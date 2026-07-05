//! App wiring: tray + popover, global shortcut, refresh loop, and the
//! invoke-handler commands the frontend calls.

mod ark_usage;
mod state;
mod tray;

use ark_usage::PlanUsage;
use state::{AppState, Settings};
use std::sync::Arc;
use std::time::Duration;
use tauri::{async_runtime, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

/// Fetch, cache, push tray title, and emit `usage-update` to the frontend.
/// Used by both the initial spawn and the periodic refresh loop.
async fn fetch_and_emit(app: &tauri::AppHandle, state: &Arc<AppState>) {
    state.set_refreshing(true);
    let usage = ark_usage::fetch().await;
    state.cache_put(usage.clone());
    let settings = state.get();
    tray::refresh_title(app, &usage, &settings);
    let _ = app.emit("usage-update", &usage);
    state.set_refreshing(false);
}

fn spawn_refresh_loop(app: tauri::AppHandle, state: Arc<AppState>) {
    async_runtime::spawn(async move {
        // Prime the cache + tray title immediately on launch.
        fetch_and_emit(&app, &state).await;
        loop {
            // Floor at 30s so a misconfigured interval can't hammer arkcli.
            let interval = state.get().refresh_interval_secs.max(30);
            tokio::time::sleep(Duration::from_secs(interval)).await;
            fetch_and_emit(&app, &state).await;
        }
    });
}

/// Toggle the popover from anywhere. Reused by the tray left-click and the
/// Ctrl+Cmd+V global shortcut so both behave identically.
fn spawn_toggle(app: &tauri::AppHandle) {
    tray::toggle_popover(app);
}

// ---- invoke-handler commands ----

#[tauri::command]
async fn get_usage(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PlanUsage, String> {
    if let Some(cached) = state.cache_get() {
        return Ok(cached);
    }
    state.set_refreshing(true);
    let usage = ark_usage::fetch().await;
    state.cache_put(usage.clone());
    let settings = state.get();
    tray::refresh_title(&app, &usage, &settings);
    state.set_refreshing(false);
    Ok(usage)
}

#[tauri::command]
async fn refresh_usage(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PlanUsage, String> {
    state.set_refreshing(true);
    let usage = ark_usage::fetch().await;
    state.cache_put(usage.clone());
    let settings = state.get();
    tray::refresh_title(&app, &usage, &settings);
    let _ = app.emit("usage-update", &usage);
    state.set_refreshing(false);
    Ok(usage)
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, Arc<AppState>>) -> Settings {
    state.get()
}

/// Persist settings, sync launch-at-login if it changed, and recompute the tray
/// title from the cached snapshot so the title reacts immediately.
#[tauri::command]
fn set_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<(), String> {
    let prev = state.get();
    if prev.autostart != settings.autostart {
        sync_autostart(&app, settings.autostart);
    }
    state.set(settings.clone());
    if let Some(usage) = state.cache_get() {
        tray::refresh_title(&app, &usage, &settings);
    }
    Ok(())
}

/// Open a Terminal window running the arkcli login. macOS-only; the
/// browser-based login flow needs an interactive terminal.
#[tauri::command]
async fn run_arkcli_login() -> Result<(), String> {
    let script = "tell application \"Terminal\" to do script \"arkcli auth login\"";
    open_terminal(script)
}

/// Open a Terminal that installs `@volcengine/ark-cli` globally and then runs
/// the login. Hardcoded command (no user input). nvm-managed node needs no
/// sudo; system node may require the user to prefix `sudo` manually.
#[tauri::command]
async fn run_arkcli_install() -> Result<(), String> {
    let script = "tell application \"Terminal\" to do script \"npm install -g @volcengine/ark-cli && arkcli auth login\"";
    open_terminal(script)
}

fn open_terminal(script: &str) -> Result<(), String> {
    let out = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| format!("打开 Terminal 失败: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

fn sync_autostart(app: &tauri::AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let currently = al.is_enabled().unwrap_or(false);
    if enabled && !currently {
        if let Err(e) = al.enable() {
            log::warn!("autostart enable failed: {}", e);
        }
    } else if !enabled && currently {
        if let Err(e) = al.disable() {
            log::warn!("autostart disable failed: {}", e);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let toggle_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyV);

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler({
                    let sc = toggle_shortcut;
                    move |app, shortcut, event| {
                        if shortcut == &sc && event.state() == ShortcutState::Pressed {
                            spawn_toggle(app);
                        }
                    }
                })
                .build(),
        )
        .manage(Arc::new(AppState::new()))
        .setup({
            let toggle = toggle_shortcut;
            move |app| {
                // Hide from Dock — this is a menu-bar accessory app.
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);

                let handle = app.handle().clone();
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                state.init_settings(&handle);

                tray::setup(&handle)?;

                // Auto-sync autostart to whatever was loaded from disk (covers the
                // case where the user toggled it in a previous session).
                sync_autostart(&handle, state.get().autostart);

                // Register the global toggle shortcut.
                {
                    use tauri_plugin_global_shortcut::GlobalShortcutExt;
                    if let Err(e) = app.global_shortcut().register(toggle) {
                        log::warn!("global shortcut register failed: {}", e);
                    }
                }

                // Standard menubar-popover behavior: hide when the window loses
                // focus. Settings is an in-webview overlay (not a system dialog),
                // so opening it never trips this blur.
                if let Some(window) = handle.get_webview_window("main") {
                    let w = window.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::Focused(false) = event {
                            let _ = w.hide();
                        }
                    });
                }

                spawn_refresh_loop(handle.clone(), state);
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_usage,
            refresh_usage,
            get_settings,
            set_settings,
            run_arkcli_login,
            run_arkcli_install,
            quit_app,
            tray::hide_popover_cmd,
            tray::set_popover_height,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

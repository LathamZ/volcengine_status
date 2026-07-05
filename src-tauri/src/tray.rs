//! Tray icon + menu, popover window positioning/show/hide, and tray-title
//! computation. The title reflects *remaining* percent (100 − used) for the
//! plans/period selected in Settings, pushed from Rust so it updates even while
//! the popover webview is hidden.

use crate::ark_usage::PlanUsage;
use crate::state::{period_label_for, Settings};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalPosition, Manager, PhysicalPosition, PhysicalSize, Runtime,
    WebviewWindow,
};

pub const POPOVER_W: f64 = 400.0;
pub const POPOVER_DEFAULT_H: f64 = 540.0;
pub const POPOVER_MIN_H: f64 = 320.0;
pub const POPOVER_MAX_H: f64 = 760.0;
pub const POPOVER_SCREEN_MARGIN: f64 = 8.0;
const POPOVER_TRAY_GAP: f64 = 6.0;

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let refresh = MenuItem::with_id(app, "refresh", "刷新", true, Some("Cmd+R"))?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, Some("Cmd+,"))?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Volcengine Status", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(app, &[&refresh, &settings, &sep, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tauri::include_image!("icons/tray-icon.png"))
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "refresh" => {
                let _ = app.emit("tray-action", "refresh");
            }
            "settings" => {
                show_popover(app);
                let _ = app.emit("tray-action", "open-settings");
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        hide_popover(app);
                    } else {
                        prepare_popover_window(&w);
                        let _ = position_window_under_tray(tray, &w);
                        let _ = w.show();
                        let _ = w.set_focus();
                        let _ = app.emit("popover-shown", ());
                    }
                }
            }
        })
        .build(app)?;

    if let Some(w) = app.get_webview_window("main") {
        prepare_popover_window(&w);
    }
    Ok(())
}

pub fn toggle_popover<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            hide_popover(app);
        } else {
            prepare_popover_window(&w);
            if let Some(tray) = app.tray_by_id("main-tray") {
                let _ = position_window_under_tray(&tray, &w);
            }
            let _ = w.show();
            let _ = w.set_focus();
            let _ = app.emit("popover-shown", ());
        }
    }
}

pub fn show_popover<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        prepare_popover_window(&w);
        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = position_window_under_tray(&tray, &w);
        }
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit("popover-shown", ());
    }
}

pub fn hide_popover<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

fn prepare_popover_window<R: Runtime>(window: &WebviewWindow<R>) {
    let _ = window.set_visible_on_all_workspaces(true);
    let _ = window.set_always_on_top(true);
}

/// Position the popover centered under the tray icon, clamped to the monitor
/// that owns the tray (not the monitor the hidden window last lived on).
fn position_window_under_tray<R: Runtime>(
    tray: &tauri::tray::TrayIcon<R>,
    window: &WebviewWindow<R>,
) -> tauri::Result<()> {
    let rect = match tray.rect()? {
        Some(r) => r,
        None => return Ok(()),
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let pos: PhysicalPosition<f64> = rect.position.to_physical(scale);
    let size: PhysicalSize<f64> = rect.size.to_physical(scale);
    let tray_x = pos.x / scale;
    let tray_y = pos.y / scale;
    let tray_w = size.width / scale;
    let tray_h = size.height / scale;

    let mut x = tray_x + (tray_w - POPOVER_W) / 2.0;
    let y = tray_y + tray_h + POPOVER_TRAY_GAP;
    let mut h = window
        .outer_size()
        .ok()
        .map(|s| s.height as f64 / scale)
        .unwrap_or(POPOVER_DEFAULT_H)
        .clamp(POPOVER_MIN_H, POPOVER_MAX_H);

    if let Ok(Some(monitor)) = window.monitor_from_point(tray_x, tray_y) {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        let m_scale = monitor.scale_factor();
        let m_x = m_pos.x as f64 / m_scale;
        let m_y = m_pos.y as f64 / m_scale;
        let m_w = m_size.width as f64 / m_scale;
        let m_h = m_size.height as f64 / m_scale;
        let max_x = m_x + m_w - POPOVER_W - 8.0;
        let min_x = m_x + 8.0;
        if x > max_x {
            x = max_x;
        }
        if x < min_x {
            x = min_x;
        }
        let available_h = m_y + m_h - y - POPOVER_SCREEN_MARGIN;
        if available_h.is_finite() && available_h > 0.0 {
            h = h.min(available_h).max(POPOVER_MIN_H.min(available_h));
        }
    }

    let _ = window.set_size(tauri::LogicalSize::new(POPOVER_W, h));
    window.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}

/// Tray title = remaining % for each selected plan/period, e.g. "A 76%  C 99%".
pub fn compute_title(usage: &PlanUsage, settings: &Settings) -> String {
    if settings.tray_plans.is_empty() || usage.plans.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for product in &settings.tray_plans {
        let prefix: &str = match product.as_str() {
            "agent-plan" => "A",
            "coding-plan" => "C",
            _ => "?",
        };
        let value = usage
            .plans
            .iter()
            .find(|p| &p.product == product)
            .and_then(|plan| {
                let label = period_label_for(product, &settings.tray_period);
                plan.periods.iter().find(|p| p.label == label)
            })
            .and_then(|p| p.remaining_percent)
            .map(|r| format!("{}%", r.round().max(0.0) as i64));
        let text = value.unwrap_or_else(|| "—".to_string());
        parts.push(format!("{} {}", prefix, text));
    }
    parts.join("  ")
}

/// Push the title to the NSStatusItem. An empty string collapses the status
/// item to icon-only; `set_title(None)` leaves a residual gap on macOS.
pub fn refresh_title(app: &AppHandle, usage: &PlanUsage, settings: &Settings) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let title = compute_title(usage, settings);
        let value: Option<String> = if title.is_empty() {
            Some(String::new())
        } else {
            Some(format!(" {}", title))
        };
        let _ = tray.set_title(value);
    }
}

#[tauri::command]
pub fn hide_popover_cmd(app: AppHandle) {
    hide_popover(&app);
}

#[tauri::command]
pub fn set_popover_height(height: f64, window: tauri::Window) -> Result<(), String> {
    let requested = height.clamp(POPOVER_MIN_H, POPOVER_MAX_H);
    let scale = window.scale_factor().unwrap_or(1.0);
    let current = window
        .outer_size()
        .map_err(|e| format!("outer_size: {}", e))?;
    let logical_w = (current.width as f64) / scale;
    let logical_h = (current.height as f64) / scale;
    if (logical_h - requested).abs() < 2.0 {
        return Ok(());
    }
    window
        .set_size(tauri::LogicalSize::new(logical_w, requested))
        .map_err(|e| format!("set_size: {}", e))?;
    Ok(())
}

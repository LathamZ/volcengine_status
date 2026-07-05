//! App state: cached usage snapshot + persisted settings.
//!
//! Settings live as a JSON file under the Tauri app-config dir so they survive
//! restarts. The cache holds the last fetch so the popover can render instantly
//! on open instead of waiting for a subprocess round-trip.

use crate::ark_usage::PlanUsage;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

/// Which periods a tray title can reflect. `short` maps to 5h (agent) / session
/// (coding) since the two products name their short window differently.
#[allow(dead_code)]
pub const PERIOD_SHORT: &str = "short";
pub const PERIOD_WEEKLY: &str = "weekly";
pub const PERIOD_MONTHLY: &str = "monthly";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub refresh_interval_secs: u64,
    /// Which plans appear in the tray title, in order. Subset of
    /// ["agent-plan", "coding-plan"].
    pub tray_plans: Vec<String>,
    /// "short" | "weekly" | "monthly" — which period the tray percent reflects.
    pub tray_period: String,
    pub threshold_warn: f64,
    pub threshold_critical: f64,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 300,
            tray_plans: vec!["agent-plan".into(), "coding-plan".into()],
            tray_period: PERIOD_MONTHLY.into(),
            threshold_warn: 70.0,
            threshold_critical: 90.0,
            autostart: false,
        }
    }
}

pub struct AppState {
    cache: RwLock<Option<CacheEntry>>,
    settings: RwLock<Settings>,
    settings_path: RwLock<Option<PathBuf>>,
    pub refreshing: AtomicBool,
}

#[derive(Clone)]
pub struct CacheEntry {
    pub data: PlanUsage,
    #[allow(dead_code)]
    pub fetched_at: DateTime<Utc>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(None),
            settings: RwLock::new(Settings::default()),
            settings_path: RwLock::new(None),
            refreshing: AtomicBool::new(false),
        }
    }

    /// Resolve the config dir from the app handle and load settings from disk
    /// (falling back to defaults on any error). Called once from `setup`.
    pub fn init_settings(&self, app: &AppHandle) {
        let dir = app
            .path()
            .app_config_dir()
            .map(|p| p.join("settings.json"))
            .ok();
        if let Some(path) = &dir {
            if let Some(parent) = dir.as_ref().and_then(|p| p.parent()) {
                let _ = std::fs::create_dir_all(parent);
            }
            *self.settings_path.write() = Some(path.clone());
            if let Ok(raw) = std::fs::read_to_string(path) {
                if let Ok(loaded) = serde_json::from_str::<Settings>(&raw) {
                    *self.settings.write() = loaded;
                }
            }
        }
        self.save();
    }

    pub fn get(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn set(&self, settings: Settings) {
        *self.settings.write() = settings;
        self.save();
    }

    fn save(&self) {
        let path = self.settings_path.read().clone();
        let Some(path) = path else { return };
        let raw = serde_json::to_string_pretty(&*self.settings.read()).unwrap_or_default();
        let _ = std::fs::write(path, raw);
    }

    pub fn cache_get(&self) -> Option<PlanUsage> {
        self.cache.read().as_ref().map(|e| e.data.clone())
    }

    pub fn cache_put(&self, data: PlanUsage) {
        let entry = CacheEntry {
            fetched_at: Utc::now(),
            data: data.clone(),
        };
        *self.cache.write() = Some(entry);
        let _ = data;
    }

    pub fn set_refreshing(&self, v: bool) {
        self.refreshing.store(v, Ordering::SeqCst);
    }
    #[allow(dead_code)]
    pub fn is_refreshing(&self) -> bool {
        self.refreshing.load(Ordering::SeqCst)
    }
}

/// Map a tray-period selector to the concrete period label for a given product.
pub fn period_label_for(product: &str, tray_period: &str) -> &'static str {
    match tray_period {
        PERIOD_WEEKLY => "weekly",
        PERIOD_MONTHLY => "monthly",
        _ => {
            if product == "coding-plan" {
                "session"
            } else {
                "5h"
            }
        }
    }
}

//! arkcli data fetching + normalization.
//!
//! Spawns the local `arkcli usage plan` subprocess (SSO-authenticated), parses
//! its JSON, and normalizes both products (agent-plan / coding-plan) into a
//! single shape the frontend can render directly. Handles the `-1` sentinel,
//! Coding Plan's percent-only periods, and auth-expired detection.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

/// Top-level payload shipped to the frontend. Always returned (never errors
/// into a Tauri `Err`) so the UI can render an auth-expired or error banner
/// from the same stream as a successful fetch.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanUsage {
    pub viewer: Viewer,
    pub plans: Vec<Plan>,
    #[serde(rename = "fetchedAt")]
    pub fetched_at: String,
    pub auth_expired: bool,
    pub not_installed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Viewer {
    pub user_name: String,
    pub account_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub product: String,
    pub edition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub periods: Vec<Period>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_text: Option<String>,
}

// ---- Raw arkcli JSON shape (snake_case as emitted by arkcli) ----

#[derive(Debug, Deserialize)]
struct RawRoot {
    #[serde(default)]
    viewer: RawViewer,
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Debug, Default, Deserialize)]
struct RawViewer {
    #[serde(default)]
    user_name: String,
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    tenant: Option<String>,
    #[serde(default)]
    region: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    product: String,
    #[serde(default)]
    edition: String,
    #[serde(default)]
    tier: Option<String>,
    #[serde(default)]
    periods: Vec<RawPeriod>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPeriod {
    #[serde(default)]
    label: String,
    #[serde(default)]
    used: Option<f64>,
    #[serde(default)]
    total: Option<f64>,
    #[serde(default)]
    percent: Option<f64>,
    #[serde(default)]
    reset_at: Option<i64>,
}

/// Fetch and normalize. Never panics; failures land in `auth_expired`/`error`.
pub async fn fetch() -> PlanUsage {
    let fetched_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    match run_and_parse().await {
        Ok((viewer, plans)) => PlanUsage {
            viewer,
            plans,
            fetched_at,
            auth_expired: false,
            not_installed: false,
            error: None,
        },
        Err(err) => {
            let auth_expired = err.is_auth_expired();
            let not_installed = matches!(err, UsageError::NotFound);
            log::warn!(
                "arkcli fetch failed (auth_expired={}, not_installed={}): {}",
                auth_expired,
                not_installed,
                err
            );
            PlanUsage {
                viewer: Viewer::default(),
                plans: Vec::new(),
                fetched_at,
                auth_expired,
                not_installed,
                // The install banner replaces the generic error for the not-found case.
                error: if not_installed {
                    None
                } else {
                    Some(err.to_string())
                },
            }
        }
    }
}

#[derive(Debug)]
enum UsageError {
    Spawn(String),
    /// Non-zero exit code; stderr retained for auth-expired sniffing.
    Failed { stderr: String, stdout: String },
    Decode(String),
    NotFound,
}

impl UsageError {
    fn is_auth_expired(&self) -> bool {
        let text = match self {
            UsageError::Failed { stderr, stdout } => {
                format!("{} {}", stderr, stdout).to_lowercase()
            }
            UsageError::Spawn(msg) => msg.to_lowercase(),
            _ => return false,
        };
        // Heuristic — refine once a real expired-session payload is captured.
        // arkcli prints something like "expired" / "login" / "401" when the SSO
        // session (48h validity) has lapsed.
        text.contains("expired")
            || text.contains("unauthorized")
            || text.contains("401")
            || text.contains("please login")
            || text.contains("please run")
            || text.contains("auth login")
            || text.contains("not authenticated")
    }
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsageError::Spawn(msg) => write!(f, "无法启动 arkcli: {}", msg),
            UsageError::NotFound => write!(f, "未找到 arkcli，请先安装并登录。"),
            UsageError::Failed { stderr, .. } => {
                let trimmed = stderr.trim();
                if trimmed.is_empty() {
                    write!(f, "arkcli 调用失败。")
                } else {
                    write!(f, "{}", trimmed)
                }
            }
            UsageError::Decode(msg) => write!(f, "解析 arkcli 输出失败: {}", msg),
        }
    }
}

async fn run_and_parse() -> Result<(Viewer, Vec<Plan>), UsageError> {
    // Resolve arkcli via PATH; surface a clean "not found" if missing.
    let output = match tokio::process::Command::new("arkcli")
        .args(["usage", "plan"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(UsageError::NotFound),
        Err(e) => return Err(UsageError::Spawn(e.to_string())),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        return Err(UsageError::Failed { stderr, stdout });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root: RawRoot = serde_json::from_str(&stdout)
        .map_err(|e| UsageError::Decode(format!("{} (开头: {:?})", e, stdout.chars().take(120).collect::<String>())))?;

    let viewer = Viewer {
        user_name: root.viewer.user_name,
        account_id: root.viewer.account_id,
        tenant: root.viewer.tenant,
        region: root.viewer.region,
    };
    let now = Utc::now();
    let plans = root
        .items
        .into_iter()
        .filter(|i| !i.periods.is_empty())
        .map(|item| Plan {
            product: item.product,
            edition: item.edition,
            tier: item.tier,
            periods: item.periods.into_iter().map(|p| normalize_period(p, now)).collect(),
        })
        .collect();
    Ok((viewer, plans))
}

/// Map a raw arkcli period onto the normalized shape: `-1` sentinels become
/// `None`, remaining% is derived, and a human reset countdown is precomputed.
fn normalize_period(raw: RawPeriod, now: DateTime<Utc>) -> Period {
    let used = raw.used.filter(|v| *v >= 0.0);
    let total = raw.total.filter(|v| *v >= 0.0);
    let percent = raw.percent.filter(|v| *v >= 0.0).map(|p| p.clamp(0.0, 100.0));
    let remaining_percent = percent.map(|p| (100.0 - p).max(0.0));
    let reset_at = raw.reset_at.filter(|v| *v > 0);
    let reset_text = reset_at.and_then(|ms| {
        DateTime::<Utc>::from_timestamp_millis(ms).map(|t| reset_text(t, now))
    });
    Period {
        label: raw.label,
        used,
        total,
        percent,
        remaining_percent,
        reset_at,
        reset_text,
    }
}

fn reset_text(reset: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (reset - now).num_seconds();
    if secs <= 0 {
        return "已重置".to_string();
    }
    let mins = (secs + 59) / 60;
    if mins < 60 {
        return format!("{} 分钟后", mins);
    }
    let hours = mins / 60;
    let rem = mins % 60;
    if hours < 48 {
        if rem > 0 {
            return format!("{} 小时 {} 分后", hours, rem);
        }
        return format!("{} 小时后", hours);
    }
    let days = hours / 24;
    let rem_hours = hours % 24;
    if rem_hours > 0 {
        format!("{} 天 {} 小时后", days, rem_hours)
    } else {
        format!("{} 天后", days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_sentinels_and_remaining() {
        let now = Utc::now();
        let p = normalize_period(
            RawPeriod {
                label: "5h".into(),
                used: Some(368.5),
                total: Some(10000.0),
                percent: Some(3.68),
                reset_at: Some(now.timestamp_millis() + 3 * 3600 * 1000),
            },
            now,
        );
        assert_eq!(p.remaining_percent.unwrap().round(), 96.0);
        assert!(p.reset_text.unwrap().contains("小时"));
    }

    #[test]
    fn drops_negative_sentinels() {
        let now = Utc::now();
        let p = normalize_period(
            RawPeriod {
                label: "session".into(),
                used: Some(-1.0),
                total: Some(-1.0),
                percent: Some(-1.0),
                reset_at: Some(-1),
            },
            now,
        );
        assert!(p.used.is_none());
        assert!(p.total.is_none());
        assert!(p.percent.is_none());
        assert!(p.reset_at.is_none());
    }

    #[test]
    fn parses_real_sample() {
        let raw = r#"{
          "viewer": {"user_name":"LathamZhao","account_id":"2104139621","tenant":"volc","region":"cn-beijing"},
          "items": [
            {"product":"agent-plan","edition":"personal","tier":"medium","subscribed":true,
             "periods":[{"label":"5h","used":368.5,"total":10000,"percent":3.68,"reset_at":1783198227000}]},
            {"product":"coding-plan","edition":"personal","subscribed":true,
             "periods":[{"label":"session","percent":2.91,"reset_at":1783202077000}]}
          ]
        }"#;
        let root: RawRoot = serde_json::from_str(raw).unwrap();
        assert_eq!(root.items.len(), 2);
        assert_eq!(root.items[0].periods[0].used, Some(368.5));
        assert!(root.items[1].periods[0].used.is_none());
    }

    #[test]
    fn detects_auth_expired_keywords() {
        let err = UsageError::Failed {
            stderr: "token expired, please run arkcli auth login".into(),
            stdout: String::new(),
        };
        assert!(err.is_auth_expired());
        let ok = UsageError::Failed { stderr: "network error".into(), stdout: String::new() };
        assert!(!ok.is_auth_expired());
    }
}

//! arkcli billing fetch + normalization.
//!
//! Spawns `arkcli billing list --billing-mode 2` (pay-as-you-go = usage
//! beyond plan quota, SSO-authenticated) and rolls the current month's
//! split-bill line items up to a per-model total the popover renders at a
//! glance. Mirrors `ark_usage`'s never-returns-Err contract: failures land in
//! `auth_expired` / `not_installed` / `error` so the frontend renders the same
//! banners from either stream.

use chrono::{Local, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ark_usage::{run_arkcli, UsageError};

/// Top-level payload shipped to the frontend. Always returned (never a Tauri
/// `Err`); same three-field failure shape as `PlanUsage`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingUsage {
    /// "YYYY-MM" - the billing month queried (current local month).
    pub bill_period: String,
    /// Σ PayableAmount across all line items, in CNY.
    pub total_amount: f64,
    pub total_records: usize,
    pub by_model: Vec<ModelUsage>,
    /// True when `--limit` truncated the month's rows.
    pub truncated: bool,
    #[serde(rename = "fetchedAt")]
    pub fetched_at: String,
    pub auth_expired: bool,
    pub not_installed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    /// arkcli `ConfigName` (e.g. "Doubao-Seed-Evolving").
    pub model: String,
    /// Σ PayableAmount for this model, CNY.
    pub amount: f64,
    /// Σ Count when every line item's `Unit` is "千tokens"; `None` if the
    /// month mixes units (e.g. image "次" calls) so we never sum across
    /// incompatible dimensions.
    pub tokens: Option<f64>,
    /// Number of split-bill line items for this model.
    pub records: usize,
}

// ---- Raw arkcli JSON shape ----
//
// Unlike `usage plan` (snake_case), `billing list` emits PascalCase field
// names, so `rename_all = "PascalCase"` maps our snake_case fields back to
// them. `PayableAmount` and `Count` arrive as strings.

#[derive(Debug, Default, Deserialize)]
struct RawBillingRoot {
    #[serde(default)]
    items: Vec<RawBillingItem>,
    #[serde(default)]
    total_records: usize,
    #[serde(default)]
    is_truncated: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawBillingItem {
    #[serde(default)]
    config_name: String,
    #[serde(default)]
    payable_amount: String,
    #[serde(default)]
    count: String,
    #[serde(default)]
    unit: String,
}

/// Fetch and normalize this month's pay-as-you-go billing. Never panics;
/// failures land in `auth_expired`/`not_installed`/`error`.
pub async fn fetch() -> BillingUsage {
    let fetched_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    // Local month so the period matches the user's billing calendar; UTC would
    // drift across the month boundary for the first 8h of each month in +08.
    let bill_period = Local::now().format("%Y-%m").to_string();
    match run_and_parse(&bill_period).await {
        Ok((total_amount, total_records, truncated, by_model)) => BillingUsage {
            bill_period,
            total_amount,
            total_records,
            truncated,
            by_model,
            fetched_at,
            auth_expired: false,
            not_installed: false,
            error: None,
        },
        Err(err) => {
            let auth_expired = err.is_auth_expired();
            let not_installed = matches!(err, UsageError::NotFound);
            log::warn!(
                "arkcli billing fetch failed (auth_expired={}, not_installed={}): {}",
                auth_expired,
                not_installed,
                err
            );
            BillingUsage {
                bill_period,
                total_amount: 0.0,
                total_records: 0,
                truncated: false,
                by_model: Vec::new(),
                fetched_at,
                auth_expired,
                not_installed,
                error: if not_installed {
                    None
                } else {
                    Some(err.to_string())
                },
            }
        }
    }
}

async fn run_and_parse(
    bill_period: &str,
) -> Result<(f64, usize, bool, Vec<ModelUsage>), UsageError> {
    // Fixed args - no user input. `--start` is the only value and it's
    // derived from the system clock, never caller-supplied (rule 2/11).
    let args = [
        "billing",
        "list",
        "--start",
        bill_period,
        "--billing-mode",
        "2",
        "--interval",
        "month",
        "--limit",
        "50",
    ];
    let stdout = run_arkcli(&args).await?;
    let root: RawBillingRoot = serde_json::from_str(&stdout).map_err(|e| {
        UsageError::Decode(format!(
            "{} (开头: {:?})",
            e,
            stdout.chars().take(120).collect::<String>()
        ))
    })?;
    let (total_amount, by_model) = aggregate(&root.items);
    Ok((
        total_amount,
        root.total_records,
        root.is_truncated,
        by_model,
    ))
}

/// Group line items by `ConfigName`, summing `PayableAmount` (always) and
/// `Count` (only while every item shares the "千tokens" unit - a mix zeroes
/// tokens rather than summing across incompatible dimensions). Pure so it
/// tests without a subprocess.
fn aggregate(items: &[RawBillingItem]) -> (f64, Vec<ModelUsage>) {
    let mut groups: BTreeMap<String, ModelAgg> = BTreeMap::new();
    let mut total_amount = 0.0f64;
    for item in items {
        let amount = item.payable_amount.parse::<f64>().unwrap_or(0.0);
        total_amount += amount;
        // `or_insert_with` because `all_tokens` starts true (optimistic) and
        // flips to false on the first non-"千tokens" unit; `Default` would
        // start it false and break the "all tokens" check.
        let agg = groups
            .entry(item.config_name.clone())
            .or_insert_with(|| ModelAgg {
                all_tokens: true,
                ..Default::default()
            });
        agg.amount += amount;
        agg.records += 1;
        if item.unit == "千tokens" {
            agg.tokens_seen = true;
            if let Ok(c) = item.count.parse::<f64>() {
                agg.tokens_sum += c;
            }
        } else {
            agg.all_tokens = false;
        }
    }
    let by_model = groups
        .into_iter()
        .map(|(model, agg)| ModelUsage {
            model,
            amount: round2(agg.amount),
            tokens: if agg.all_tokens && agg.tokens_seen {
                Some(round2(agg.tokens_sum))
            } else {
                None
            },
            records: agg.records,
        })
        .collect();
    (round2(total_amount), by_model)
}

#[derive(Default)]
struct ModelAgg {
    amount: f64,
    records: usize,
    tokens_sum: f64,
    tokens_seen: bool,
    all_tokens: bool,
}

/// Round to 2 decimals to absorb float drift from summing string amounts.
fn round2(n: f64) -> f64 {
    (n * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_real_sample_by_model() {
        // Captured from `arkcli billing list --billing-mode 2 --interval month`:
        // three line items, all Doubao-Seed-Evolving on one endpoint, split by
        // input / KVcache-hit / output charge items.
        let raw = r#"{"items":[
          {"ConfigName":"Doubao-Seed-Evolving","PayableAmount":"0.77","Count":"201.49","Unit":"千tokens","BillPeriod":"2026-07"},
          {"ConfigName":"Doubao-Seed-Evolving","PayableAmount":"3.21","Count":"3107.904","Unit":"千tokens","BillPeriod":"2026-07"},
          {"ConfigName":"Doubao-Seed-Evolving","PayableAmount":"0.66","Count":"25.067","Unit":"千tokens","BillPeriod":"2026-07"}
        ]}"#;
        let root: RawBillingRoot = serde_json::from_str(raw).unwrap();
        let (total, by_model) = aggregate(&root.items);
        assert!((total - 4.64).abs() < 0.001);
        assert_eq!(by_model.len(), 1);
        let m = &by_model[0];
        assert_eq!(m.model, "Doubao-Seed-Evolving");
        assert!((m.amount - 4.64).abs() < 0.001);
        assert_eq!(m.records, 3);
        assert!((m.tokens.unwrap() - 3334.46).abs() < 0.01);
    }

    #[test]
    fn parses_pascal_case_fields_as_strings() {
        let raw = r#"{"items":[
          {"ConfigName":"M","PayableAmount":"1.005","Count":"42","Unit":"千tokens"}
        ]}"#;
        let root: RawBillingRoot = serde_json::from_str(raw).unwrap();
        assert_eq!(root.items[0].config_name, "M");
        assert_eq!(root.items[0].payable_amount, "1.005");
        assert_eq!(root.items[0].count, "42");
        assert_eq!(root.items[0].unit, "千tokens");
    }

    #[test]
    fn aggregates_multiple_models_sorted() {
        let raw = r#"{"items":[
          {"ConfigName":"Model-B","PayableAmount":"2.00","Count":"50","Unit":"千tokens"},
          {"ConfigName":"Model-A","PayableAmount":"0.50","Count":"100","Unit":"千tokens"},
          {"ConfigName":"Model-A","PayableAmount":"1.50","Count":"200","Unit":"千tokens"}
        ]}"#;
        let root: RawBillingRoot = serde_json::from_str(raw).unwrap();
        let (total, by_model) = aggregate(&root.items);
        assert!((total - 4.0).abs() < 0.001);
        // BTreeMap -> sorted: Model-A first.
        assert_eq!(by_model[0].model, "Model-A");
        assert!((by_model[0].amount - 2.0).abs() < 0.001);
        assert_eq!(by_model[0].records, 2);
        assert!((by_model[1].amount - 2.0).abs() < 0.001);
    }

    #[test]
    fn tokens_none_when_units_mixed() {
        let raw = r#"{"items":[
          {"ConfigName":"M","PayableAmount":"1.00","Count":"100","Unit":"千tokens"},
          {"ConfigName":"M","PayableAmount":"2.00","Count":"5","Unit":"次"}
        ]}"#;
        let root: RawBillingRoot = serde_json::from_str(raw).unwrap();
        let (total, by_model) = aggregate(&root.items);
        assert!((total - 3.0).abs() < 0.001);
        assert_eq!(by_model.len(), 1);
        // Mixed 千tokens / 次 -> tokens suppressed (amount still summed).
        assert_eq!(by_model[0].tokens, None);
        assert_eq!(by_model[0].records, 2);
    }

    #[test]
    fn handles_empty_items_and_missing_fields() {
        let (total, by_model) = aggregate(&[]);
        assert!((total - 0.0).abs() < 0.001);
        assert!(by_model.is_empty());

        // Missing fields default; amount parses to 0.0 on bad strings.
        let raw = r#"{"items":[{"PayableAmount":"not-a-number"},{"ConfigName":"M"}]}"#;
        let root: RawBillingRoot = serde_json::from_str(raw).unwrap();
        let (total, by_model) = aggregate(&root.items);
        assert!((total - 0.0).abs() < 0.001);
        assert_eq!(by_model.len(), 2); // "" and "M"
    }

    #[test]
    fn round2_absorbs_float_drift() {
        // 0.1 + 0.2 = 0.30000000000000004 in f64; round2 snaps it back.
        assert!((round2(0.1 + 0.2) - 0.3).abs() < 0.0001);
        // Non-half values round predictably. (1.005 would too in math, but f64
        // stores it as 1.00499... so round lands on 1.0 - amounts arrive as
        // 2-decimal strings so this edge never occurs in practice.)
        assert!((round2(0.126) - 0.13).abs() < 0.0001);
    }
}

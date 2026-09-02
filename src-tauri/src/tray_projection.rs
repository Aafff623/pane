use crate::providers;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrayProjectionConfig {
    pub disabled: Vec<String>,
    pub provider_order: Vec<String>,
    pub providers: HashMap<String, ProviderProjectionConfig>,
    pub pinned: Option<PinnedMetric>,
    pub locale: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderProjectionConfig {
    pub metric_order: Vec<String>,
    pub hidden: Vec<String>,
    pub starred: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PinnedMetric {
    pub provider: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainTrayIconMode {
    Logo,
    Numbers,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainTrayProjection {
    pub icon_mode: MainTrayIconMode,
    pub remaining_percentages: Vec<u32>,
    pub tooltip: String,
}

/// One/New API is two-level: family id `onenewapi` hides every key card.
/// Claude/Codex extra accounts stay independent of the bare family id.
fn projection_disabled(id: &str, disabled: &[String]) -> bool {
    if disabled.iter().any(|d| d == id) {
        return true;
    }
    let family = id.split('@').next().unwrap_or(id);
    family == "onenewapi" && disabled.iter().any(|d| d == "onenewapi")
}

pub(crate) fn project_main_tray(
    snapshots: &[providers::Snapshot],
    config: &TrayProjectionConfig,
    strip_active: bool,
) -> MainTrayProjection {
    let visible: Vec<&providers::Snapshot> = config
        .provider_order
        .iter()
        .filter(|provider_id| !projection_disabled(provider_id, &config.disabled))
        .filter_map(|provider_id| {
            snapshots
                .iter()
                .find(|snapshot| snapshot.id == *provider_id)
        })
        .filter(|snapshot| {
            snapshot.status == "ok"
                && snapshot
                    .metrics
                    .iter()
                    .any(|metric| metric.kind == "progress")
        })
        .collect();
    let active_pinned = config.pinned.as_ref().and_then(|pinned| {
        let snapshot = visible
            .iter()
            .find(|snapshot| snapshot.id == pinned.provider)
            .copied()?;
        let metric = snapshot
            .metrics
            .iter()
            .find(|metric| metric.kind == "progress" && metric.label == pinned.label)?;
        Some((snapshot, metric))
    });
    let auto_provider = visible.iter().copied().find(|snapshot| {
        !ordinary_metrics(snapshot, config.providers.get(&snapshot.id)).is_empty()
    });
    let Some(icon_provider) = active_pinned
        .map(|(snapshot, _)| snapshot)
        .or(auto_provider)
    else {
        return MainTrayProjection {
            icon_mode: MainTrayIconMode::Logo,
            remaining_percentages: Vec::new(),
            tooltip: "Pane".into(),
        };
    };
    let mut icon_metrics = ordinary_metrics(icon_provider, config.providers.get(&icon_provider.id));
    if let Some((_, pinned_metric)) = active_pinned {
        icon_metrics.retain(|metric| metric.label != pinned_metric.label);
        icon_metrics.insert(0, pinned_metric);
    }
    icon_metrics.truncate(2);

    let locale = serde_json::json!({ "locale": config.locale });
    let pinned_provider_id = active_pinned.map(|(snapshot, _)| snapshot.id.as_str());
    let displayable: Vec<&providers::Snapshot> = visible
        .iter()
        .copied()
        .filter(|snapshot| {
            Some(snapshot.id.as_str()) == pinned_provider_id
                || !ordinary_metrics(snapshot, config.providers.get(&snapshot.id)).is_empty()
        })
        .collect();
    let mut tooltip_providers: Vec<&providers::Snapshot> =
        displayable.iter().copied().take(6).collect();
    if let Some((pinned_provider, _)) = active_pinned {
        if !tooltip_providers
            .iter()
            .any(|snapshot| snapshot.id == pinned_provider.id)
        {
            tooltip_providers.truncate(5);
            tooltip_providers.push(pinned_provider);
        }
    }
    let mut tooltip_lines: Vec<(&str, String)> = Vec::new();
    for snapshot in tooltip_providers {
        let mut metrics = if snapshot.id == icon_provider.id {
            icon_metrics.clone()
        } else {
            ordinary_metrics(snapshot, config.providers.get(&snapshot.id))
        };
        metrics.truncate(if snapshot.id == icon_provider.id {
            2
        } else {
            1
        });
        if metrics.is_empty() {
            continue;
        }
        tooltip_lines.push((
            &snapshot.id,
            format_provider_line(snapshot, &metrics, &locale),
        ));
    }
    while tooltip_utf16_len(&tooltip_lines) > 127 {
        let pinned_id = active_pinned.map(|(snapshot, _)| snapshot.id.as_str());
        let remove_at = tooltip_lines
            .iter()
            .rposition(|(provider_id, _)| Some(*provider_id) != pinned_id)
            .or_else(|| tooltip_lines.len().checked_sub(1));
        let Some(remove_at) = remove_at else { break };
        tooltip_lines.remove(remove_at);
    }
    let tooltip = tooltip_from_lines(&tooltip_lines);
    let icon_explained = tooltip_lines
        .iter()
        .any(|(provider_id, _)| *provider_id == icon_provider.id.as_str());
    let show_numbers = !strip_active && icon_explained;

    MainTrayProjection {
        icon_mode: if show_numbers {
            MainTrayIconMode::Numbers
        } else {
            MainTrayIconMode::Logo
        },
        remaining_percentages: if show_numbers {
            icon_metrics
                .into_iter()
                .map(|metric| percent_left(metric) as u32)
                .collect()
        } else {
            Vec::new()
        },
        tooltip,
    }
}

fn tooltip_from_lines(lines: &[(&str, String)]) -> String {
    let mut tooltip = String::from("Pane");
    for (_, line) in lines {
        tooltip.push('\n');
        tooltip.push_str(line);
    }
    tooltip
}

fn tooltip_utf16_len(lines: &[(&str, String)]) -> usize {
    4 + lines
        .iter()
        .map(|(_, line)| 1 + line.encode_utf16().count())
        .sum::<usize>()
}

fn ordinary_metrics<'a>(
    snapshot: &'a providers::Snapshot,
    config: Option<&ProviderProjectionConfig>,
) -> Vec<&'a providers::Metric> {
    let hidden = config
        .map(|value| value.hidden.as_slice())
        .unwrap_or_default();
    let mut labels: Vec<&str> = Vec::new();
    if let Some(config) = config {
        labels.extend(config.starred.iter().map(String::as_str));
        labels.extend(config.metric_order.iter().map(String::as_str));
    }
    labels.extend(snapshot.metrics.iter().map(|metric| metric.label.as_str()));

    let mut metrics = Vec::new();
    for label in labels {
        if hidden.iter().any(|hidden_label| hidden_label == label)
            || metrics
                .iter()
                .any(|metric: &&providers::Metric| metric.label == label)
        {
            continue;
        }
        if let Some(metric) = snapshot
            .metrics
            .iter()
            .find(|metric| metric.kind == "progress" && metric.label == label)
        {
            metrics.push(metric);
        }
    }
    metrics
}

fn format_provider_line(
    snapshot: &providers::Snapshot,
    metrics: &[&providers::Metric],
    locale: &serde_json::Value,
) -> String {
    let name = if snapshot.stale {
        format!("⚠ {}", snapshot.name)
    } else {
        snapshot.name.clone()
    };
    let mut metrics = metrics.iter();
    let Some(first) = metrics.next() else {
        return name;
    };
    let mut line = crate::i18n::pct_left(locale, &name, &first.label, percent_left(first));
    let resolved = crate::i18n::resolved_locale(locale);
    let separator = if resolved == "zh" { "，" } else { ", " };
    for metric in metrics {
        let fragment = {
            let label = crate::i18n::metric_label(locale, &metric.label);
            match resolved {
                "zh" => format!("{label}: 剩余 {:.0}%", percent_left(metric)),
                "ru" => format!("{label}: осталось {:.0}%", percent_left(metric)),
                _ => format!("{label}: {:.0}% left", percent_left(metric)),
            }
        };
        line.push_str(separator);
        line.push_str(&fragment);
    }
    line
}

fn percent_left(metric: &providers::Metric) -> f64 {
    (100.0 - metric.used_percent.unwrap_or(0.0))
        .clamp(0.0, 100.0)
        .round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{Metric, Snapshot};

    fn progress(label: &str, used: f64) -> Metric {
        Metric::progress(label, used, None)
    }

    fn snapshot(id: &str, name: &str, metrics: Vec<Metric>) -> Snapshot {
        Snapshot::ok(id, name, None, metrics)
    }

    fn provider(metric_order: &[&str]) -> ProviderProjectionConfig {
        ProviderProjectionConfig {
            metric_order: metric_order
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            ..Default::default()
        }
    }

    fn config(order: &[&str]) -> TrayProjectionConfig {
        TrayProjectionConfig {
            disabled: Vec::new(),
            provider_order: order.iter().map(|value| (*value).to_string()).collect(),
            providers: HashMap::new(),
            pinned: None,
            locale: "en".into(),
        }
    }

    #[test]
    fn auto_projects_two_numbers_and_a_provider_tooltip() {
        let snapshots = vec![snapshot(
            "codex",
            "Codex",
            vec![progress("Session", 18.0), progress("Weekly", 63.0)],
        )];
        let mut cfg = config(&["codex"]);
        cfg.providers
            .insert("codex".into(), provider(&["Session", "Weekly"]));

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.icon_mode, MainTrayIconMode::Numbers);
        assert_eq!(result.remaining_percentages, vec![82, 37]);
        assert_eq!(
            result.tooltip,
            "Pane\nCodex Session: 82% left, Weekly: 37% left"
        );
    }

    #[test]
    fn provider_order_drives_auto_numbers_and_tooltip_order() {
        let snapshots = vec![
            snapshot("claude", "Claude", vec![progress("Session", 70.0)]),
            snapshot("codex", "Codex", vec![progress("Weekly", 15.0)]),
        ];
        let mut cfg = config(&["codex", "claude"]);
        cfg.providers
            .insert("claude".into(), provider(&["Session"]));
        cfg.providers.insert("codex".into(), provider(&["Weekly"]));

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![85]);
        assert_eq!(
            result.tooltip,
            "Pane\nCodex Weekly: 85% left\nClaude Session: 30% left"
        );
    }

    #[test]
    fn disabled_account_is_excluded_without_disabling_its_provider_family() {
        let snapshots = vec![
            snapshot(
                "claude@home",
                "Claude Home",
                vec![progress("Session", 25.0)],
            ),
            snapshot(
                "claude@work",
                "Claude Work",
                vec![progress("Session", 60.0)],
            ),
        ];
        let mut cfg = config(&["claude@home", "claude@work"]);
        cfg.disabled.push("claude@home".into());

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![40]);
        assert_eq!(result.tooltip, "Pane\nClaude Work Session: 40% left");
        assert_eq!(cfg.provider_order, vec!["claude@home", "claude@work"]);
    }

    #[test]
    fn starred_then_visible_metric_order_drives_auto_selection() {
        let snapshots = vec![snapshot(
            "codex",
            "Codex",
            vec![
                progress("Session", 20.0),
                progress("Weekly", 40.0),
                progress("Monthly", 60.0),
                progress("New", 70.0),
            ],
        )];
        let mut cfg = config(&["codex"]);
        let mut layout = provider(&["Weekly", "Session"]);
        layout.starred = vec!["Monthly".into()];
        layout.hidden = vec!["Weekly".into()];
        cfg.providers.insert("codex".into(), layout);

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![40, 80]);
        assert_eq!(
            result.tooltip,
            "Pane\nCodex Monthly: 40% left, Session: 80% left"
        );
    }

    #[test]
    fn pinned_metric_overrides_hidden_and_starred_selection() {
        let snapshots = vec![
            snapshot("codex", "Codex", vec![progress("Session", 10.0)]),
            snapshot(
                "claude",
                "Claude",
                vec![progress("Session", 25.0), progress("Weekly", 55.0)],
            ),
        ];
        let mut cfg = config(&["codex", "claude"]);
        let mut claude = provider(&["Session", "Weekly"]);
        claude.starred = vec!["Session".into()];
        claude.hidden = vec!["Weekly".into()];
        cfg.providers.insert("claude".into(), claude);
        cfg.pinned = Some(PinnedMetric {
            provider: "claude".into(),
            label: "Weekly".into(),
        });

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![45, 75]);
        assert_eq!(
            result.tooltip,
            "Pane\nCodex Session: 90% left\nClaude Weekly: 45% left, Session: 75% left"
        );
    }

    #[test]
    fn pinned_provider_outside_first_six_replaces_the_last_normal_provider() {
        let snapshots: Vec<Snapshot> = (1..=7)
            .map(|n| {
                snapshot(
                    &format!("p{n}"),
                    &format!("P{n}"),
                    vec![progress("Usage", n as f64)],
                )
            })
            .collect();
        let order: Vec<String> = (1..=7).map(|n| format!("p{n}")).collect();
        let order_refs: Vec<&str> = order.iter().map(String::as_str).collect();
        let mut cfg = config(&order_refs);
        cfg.pinned = Some(PinnedMetric {
            provider: "p7".into(),
            label: "Usage".into(),
        });

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![93]);
        assert!(result.tooltip.contains("\nP5 Usage: 95% left"));
        assert!(!result.tooltip.contains("\nP6 Usage"));
        assert!(result.tooltip.contains("\nP7 Usage: 93% left"));
        assert_eq!(result.tooltip.lines().count(), 7);
    }

    #[test]
    fn stale_last_success_keeps_numbers_and_marks_the_complete_provider_line() {
        let mut stale = snapshot("codex", "Codex", vec![progress("Weekly", 34.0)]);
        stale.stale = true;
        stale.warning = Some("timeout".into());
        let cfg = config(&["codex"]);

        let result = project_main_tray(&[stale], &cfg, false);

        assert_eq!(result.remaining_percentages, vec![66]);
        assert_eq!(result.tooltip, "Pane\n⚠ Codex Weekly: 66% left");
    }

    #[test]
    fn tooltip_capacity_counts_utf16_and_only_keeps_complete_lines() {
        let snapshots: Vec<Snapshot> = (1..=6)
            .map(|n| {
                snapshot(
                    &format!("p{n}"),
                    &format!("供应商😀😀😀{n}"),
                    vec![progress("Weekly", 50.0)],
                )
            })
            .collect();
        let order: Vec<String> = (1..=6).map(|n| format!("p{n}")).collect();
        let order_refs: Vec<&str> = order.iter().map(String::as_str).collect();

        let result = project_main_tray(&snapshots, &config(&order_refs), false);

        assert!(result.tooltip.encode_utf16().count() <= 127);
        for line in result.tooltip.lines().skip(1) {
            assert!(line.ends_with("Weekly: 50% left"), "partial line: {line}");
        }
        assert!(result.tooltip.lines().count() < 7);
    }

    #[test]
    fn tooltip_keeps_a_complete_line_at_the_exact_utf16_limit() {
        let provider_name = "a".repeat(106);
        let snapshots = vec![snapshot(
            "p1",
            &provider_name,
            vec![progress("Usage", 50.0)],
        )];

        let result = project_main_tray(&snapshots, &config(&["p1"]), false);

        assert_eq!(result.tooltip.encode_utf16().count(), 127);
        assert!(result.tooltip.ends_with("Usage: 50% left"));
    }

    #[test]
    fn tooltip_drops_a_whole_line_when_stale_marker_pushes_it_over_capacity() {
        let mut stale = snapshot("p1", &"a".repeat(106), vec![progress("Usage", 50.0)]);
        stale.stale = true;

        let result = project_main_tray(&[stale], &config(&["p1"]), false);

        assert_eq!(result.tooltip, "Pane");
        assert_eq!(result.icon_mode, MainTrayIconMode::Logo);
        assert!(result.remaining_percentages.is_empty());
    }

    #[test]
    fn tooltip_capacity_keeps_a_complete_two_metric_line_at_the_utf16_limit() {
        let snapshots = vec![snapshot(
            "p1",
            &"a".repeat(86),
            vec![progress("Session", 50.0), progress("Weekly", 50.0)],
        )];

        let result = project_main_tray(&snapshots, &config(&["p1"]), false);

        assert_eq!(result.tooltip.encode_utf16().count(), 127);
        assert_eq!(
            result.tooltip,
            format!(
                "Pane\n{} Session: 50% left, Weekly: 50% left",
                "a".repeat(86)
            )
        );
        assert_eq!(result.icon_mode, MainTrayIconMode::Numbers);
        assert_eq!(result.remaining_percentages, vec![50, 50]);
    }

    #[test]
    fn tooltip_capacity_drops_an_overlong_two_metric_line_and_hides_numbers() {
        let snapshots = vec![snapshot(
            "p1",
            &"a".repeat(87),
            vec![progress("Session", 50.0), progress("Weekly", 50.0)],
        )];

        let result = project_main_tray(&snapshots, &config(&["p1"]), false);

        assert_eq!(result.tooltip, "Pane");
        assert_eq!(result.icon_mode, MainTrayIconMode::Logo);
        assert!(result.remaining_percentages.is_empty());
    }

    #[test]
    fn tooltip_capacity_reserves_the_active_pinned_provider_line() {
        let snapshots: Vec<Snapshot> = (1..=7)
            .map(|n| {
                snapshot(
                    &format!("p{n}"),
                    &format!("Provider-{n}-with-a-long-name"),
                    vec![progress("Usage", 50.0)],
                )
            })
            .collect();
        let order: Vec<String> = (1..=7).map(|n| format!("p{n}")).collect();
        let order_refs: Vec<&str> = order.iter().map(String::as_str).collect();
        let mut cfg = config(&order_refs);
        cfg.pinned = Some(PinnedMetric {
            provider: "p7".into(),
            label: "Usage".into(),
        });

        let result = project_main_tray(&snapshots, &cfg, false);

        assert!(result
            .tooltip
            .contains("Provider-7-with-a-long-name Usage: 50% left"));
        assert!(result.tooltip.encode_utf16().count() <= 127);
    }

    #[test]
    fn locale_changes_text_without_changing_selection() {
        let snapshots = vec![snapshot(
            "codex",
            "Codex",
            vec![progress("Weekly", 23.0), progress("Session", 10.0)],
        )];
        let en = project_main_tray(&snapshots, &config(&["codex"]), false);
        let mut zh_config = config(&["codex"]);
        zh_config.locale = "zh".into();
        let mut ru_config = config(&["codex"]);
        ru_config.locale = "ru".into();

        let zh = project_main_tray(&snapshots, &zh_config, false);
        let ru = project_main_tray(&snapshots, &ru_config, false);

        assert_eq!(en.remaining_percentages, zh.remaining_percentages);
        assert_eq!(en.remaining_percentages, ru.remaining_percentages);
        assert_eq!(zh.tooltip, "Pane\nCodex 每周: 剩余 77%，会话: 剩余 90%");
        assert_eq!(
            ru.tooltip,
            "Pane\nCodex За неделю: осталось 77%, Сессия: осталось 90%"
        );
    }

    #[test]
    fn actual_strip_entries_switch_main_icon_to_logo() {
        let snapshots = vec![snapshot("codex", "Codex", vec![progress("Weekly", 23.0)])];

        let result = project_main_tray(&snapshots, &config(&["codex"]), true);

        assert_eq!(result.icon_mode, MainTrayIconMode::Logo);
        assert!(result.remaining_percentages.is_empty());
        assert_eq!(result.tooltip, "Pane\nCodex Weekly: 77% left");
    }

    #[test]
    fn providers_without_a_usable_progress_snapshot_produce_the_empty_state() {
        let snapshots = vec![
            Snapshot::error("codex", "Codex", "offline".into()),
            Snapshot::no_credentials("grok", "Grok", "login"),
            snapshot("claude", "Claude", vec![Metric::text("Plan", "Pro".into())]),
        ];

        let result = project_main_tray(&snapshots, &config(&["codex", "grok", "claude"]), false);

        assert_eq!(result.icon_mode, MainTrayIconMode::Logo);
        assert!(result.remaining_percentages.is_empty());
        assert_eq!(result.tooltip, "Pane");
    }

    #[test]
    fn inactive_pin_sleeps_while_disabled_and_restores_when_reenabled() {
        let snapshots = vec![
            snapshot("codex", "Codex", vec![progress("Session", 10.0)]),
            snapshot("claude", "Claude", vec![progress("Weekly", 70.0)]),
        ];
        let mut cfg = config(&["codex", "claude"]);
        cfg.pinned = Some(PinnedMetric {
            provider: "claude".into(),
            label: "Weekly".into(),
        });
        cfg.disabled.push("claude".into());

        let sleeping = project_main_tray(&snapshots, &cfg, false);
        cfg.disabled.clear();
        let restored = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(sleeping.remaining_percentages, vec![90]);
        assert_eq!(restored.remaining_percentages, vec![30]);
        assert_eq!(cfg.pinned.as_ref().unwrap().provider, "claude");
    }

    #[test]
    fn auto_skips_a_provider_when_all_of_its_metrics_are_hidden() {
        let snapshots = vec![
            snapshot("codex", "Codex", vec![progress("Session", 10.0)]),
            snapshot("claude", "Claude", vec![progress("Weekly", 70.0)]),
        ];
        let mut cfg = config(&["codex", "claude"]);
        let mut codex = provider(&["Session"]);
        codex.hidden.push("Session".into());
        cfg.providers.insert("codex".into(), codex);

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![30]);
        assert_eq!(result.tooltip, "Pane\nClaude Weekly: 30% left");
    }

    #[test]
    fn newly_seen_visible_progress_metric_follows_the_saved_visible_order() {
        let snapshots = vec![snapshot(
            "codex",
            "Codex",
            vec![progress("Old", 10.0), progress("New", 35.0)],
        )];
        let mut cfg = config(&["codex"]);
        let mut codex = provider(&["Old"]);
        codex.hidden.push("Old".into());
        cfg.providers.insert("codex".into(), codex);

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![65]);
        assert_eq!(result.tooltip, "Pane\nCodex New: 65% left");
    }

    #[test]
    fn normal_tooltip_never_selects_more_than_six_providers() {
        let snapshots: Vec<Snapshot> = (1..=7)
            .map(|n| {
                snapshot(
                    &format!("p{n}"),
                    &format!("P{n}"),
                    vec![progress("Usage", 50.0)],
                )
            })
            .collect();
        let order: Vec<String> = (1..=7).map(|n| format!("p{n}")).collect();
        let order_refs: Vec<&str> = order.iter().map(String::as_str).collect();

        let result = project_main_tray(&snapshots, &config(&order_refs), false);

        assert_eq!(result.tooltip.lines().count(), 7);
        assert!(!result.tooltip.contains("\nP7 Usage"));
    }

    #[test]
    fn reenabled_provider_returns_to_its_saved_position() {
        let snapshots = vec![
            snapshot("codex", "Codex", vec![progress("Session", 10.0)]),
            snapshot("claude", "Claude", vec![progress("Weekly", 70.0)]),
        ];
        let mut cfg = config(&["codex", "claude"]);
        cfg.disabled.push("codex".into());
        let disabled = project_main_tray(&snapshots, &cfg, false);

        cfg.disabled.clear();
        let reenabled = project_main_tray(&snapshots, &cfg, false);

        assert!(disabled.tooltip.starts_with("Pane\nClaude"));
        assert!(reenabled.tooltip.starts_with("Pane\nCodex"));
        assert_eq!(cfg.provider_order, vec!["codex", "claude"]);
    }

    #[test]
    fn stale_marker_is_scoped_to_the_matching_multi_account_snapshot() {
        let mut work = snapshot("claude@work", "Claude Work", vec![progress("Weekly", 30.0)]);
        work.stale = true;
        let snapshots = vec![
            snapshot("claude@home", "Claude Home", vec![progress("Weekly", 20.0)]),
            work,
        ];

        let result = project_main_tray(&snapshots, &config(&["claude@home", "claude@work"]), false);

        assert!(result.tooltip.contains("\nClaude Home Weekly: 80% left"));
        assert!(result.tooltip.contains("\n⚠ Claude Work Weekly: 70% left"));
    }

    #[test]
    fn onenewapi_family_disable_excludes_every_key_card() {
        let snapshots = vec![
            snapshot(
                "onenewapi@k1",
                "Site · Key 1",
                vec![progress("Usage", 20.0)],
            ),
            snapshot(
                "onenewapi@k2",
                "Site · Key 2",
                vec![progress("Usage", 40.0)],
            ),
        ];
        let mut cfg = config(&["onenewapi@k1", "onenewapi@k2"]);
        cfg.disabled.push("onenewapi".into());

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.icon_mode, MainTrayIconMode::Logo);
        assert!(result.remaining_percentages.is_empty());
        assert_eq!(result.tooltip, "Pane");
        assert_eq!(cfg.provider_order, vec!["onenewapi@k1", "onenewapi@k2"]);
    }

    #[test]
    fn onenewapi_per_key_disable_leaves_the_other_key_in_saved_order() {
        let snapshots = vec![
            snapshot(
                "onenewapi@k1",
                "Site · Key 1",
                vec![progress("Usage", 20.0)],
            ),
            snapshot("codex", "Codex", vec![progress("Session", 10.0)]),
            snapshot(
                "onenewapi@k2",
                "Site · Key 2",
                vec![progress("Usage", 40.0)],
            ),
        ];
        let mut cfg = config(&["onenewapi@k2", "codex", "onenewapi@k1"]);
        cfg.disabled.push("onenewapi@k1".into());

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![60]);
        assert_eq!(
            result.tooltip,
            "Pane\nSite · Key 2 Usage: 60% left\nCodex Session: 90% left"
        );
        assert_eq!(
            cfg.provider_order,
            vec!["onenewapi@k2", "codex", "onenewapi@k1"]
        );
    }

    #[test]
    fn claude_family_disable_does_not_exclude_account_cards() {
        let snapshots = vec![
            snapshot(
                "claude@home",
                "Claude Home",
                vec![progress("Session", 25.0)],
            ),
            snapshot(
                "claude@work",
                "Claude Work",
                vec![progress("Session", 60.0)],
            ),
        ];
        let mut cfg = config(&["claude@home", "claude@work"]);
        cfg.disabled.push("claude".into());

        let result = project_main_tray(&snapshots, &cfg, false);

        assert_eq!(result.remaining_percentages, vec![75]);
        assert!(result.tooltip.contains("Claude Home Session: 75% left"));
        assert!(result.tooltip.contains("Claude Work Session: 40% left"));
    }

    #[test]
    fn tooltip_keeps_a_complete_line_just_below_the_utf16_limit() {
        let provider_name = "a".repeat(105);
        let snapshots = vec![snapshot(
            "p1",
            &provider_name,
            vec![progress("Usage", 50.0)],
        )];

        let result = project_main_tray(&snapshots, &config(&["p1"]), false);

        assert_eq!(result.tooltip.encode_utf16().count(), 126);
        assert!(result.tooltip.ends_with("Usage: 50% left"));
    }
}

//! UI locale for tray + Windows toasts. The popover translates in TypeScript;
//! these strings have to live here because they are painted by Rust.

use serde_json::Value;

pub fn resolved_locale(cfg: &Value) -> &'static str {
    match cfg.get("locale").and_then(Value::as_str) {
        Some("zh") => "zh",
        Some("en") => "en",
        _ => {
            if system_locale_is_zh() {
                "zh"
            } else {
                "en"
            }
        }
    }
}

pub fn is_zh(cfg: &Value) -> bool {
    resolved_locale(cfg) == "zh"
}

pub fn quit_label(cfg: &Value) -> &'static str {
    if is_zh(cfg) {
        "退出 Pane"
    } else {
        "Quit Pane"
    }
}

pub fn metric_label(cfg: &Value, label: &str) -> String {
    if !is_zh(cfg) {
        return label.to_string();
    }
    match label {
        "Session" => "会话".into(),
        "Weekly" => "每周".into(),
        "Monthly" => "每月".into(),
        "Daily" => "每天".into(),
        "Usage" => "用量".into(),
        "Credits" => "额度".into(),
        "Credits used" => "已用额度".into(),
        "API" => "API".into(),
        "Balance" => "余额".into(),
        "Vouchers" => "代金券".into(),
        "Cash" => "现金".into(),
        "Limit" => "上限".into(),
        "Used" => "已用".into(),
        "On-demand" => "按量".into(),
        "Cursor Models" => "Cursor 模型".into(),
        "Other Models" => "其他模型".into(),
        "Total usage" => "总用量".into(),
        "Bonus" => "赠送".into(),
        "Extra usage" => "额外用量".into(),
        "Extra credits" => "额外额度".into(),
        "Reset credits" => "重置额度".into(),
        "Extra balance" => "额外余额".into(),
        "Kilo Pass" => "Kilo Pass".into(),
        "Requests today" => "今日请求".into(),
        "Requests this month" => "本月请求".into(),
        "Requests this cycle" => "本周期请求".into(),
        "Last used" => "上次使用".into(),
        "Via" => "经由".into(),
        "Sessions" => "会话数".into(),
        other if other.ends_with(" weekly") => {
            format!("{} 每周", other.trim_end_matches(" weekly"))
        }
        other => other.to_string(),
    }
}

pub fn pct_left(cfg: &Value, name: &str, label: &str, left: f64) -> String {
    let shown = metric_label(cfg, label);
    if is_zh(cfg) {
        format!("{name} {shown}: 剩余 {left:.0}%")
    } else {
        format!("{name} {shown}: {left:.0}% left")
    }
}

fn system_locale_is_zh() -> bool {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if n <= 1 {
        return false;
    }
    let s = String::from_utf16_lossy(&buf[..(n as usize - 1)]);
    s.to_ascii_lowercase().starts_with("zh")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explicit_locale_wins() {
        assert_eq!(resolved_locale(&json!({"locale": "zh"})), "zh");
        assert_eq!(resolved_locale(&json!({"locale": "en"})), "en");
    }

    #[test]
    fn zh_metric_labels() {
        let zh = json!({"locale": "zh"});
        assert_eq!(metric_label(&zh, "Session"), "会话");
        assert_eq!(metric_label(&zh, "Sonnet weekly"), "Sonnet 每周");
        assert_eq!(metric_label(&json!({"locale": "en"}), "Session"), "Session");
    }
}

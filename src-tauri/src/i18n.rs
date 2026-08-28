//! UI locale for tray + Windows toasts. The popover translates in TypeScript;
//! these strings have to live here because they are painted by Rust.

use serde_json::Value;

pub fn resolved_locale(cfg: &Value) -> &'static str {
    match cfg.get("locale").and_then(Value::as_str) {
        Some("zh") => "zh",
        Some("ru") => "ru",
        Some("en") => "en",
        _ => system_ui_locale(),
    }
}

pub fn quit_label(cfg: &Value) -> &'static str {
    match resolved_locale(cfg) {
        "zh" => "退出 Pane",
        "ru" => "Выйти из Pane",
        _ => "Quit Pane",
    }
}

pub fn metric_label(cfg: &Value, label: &str) -> String {
    match resolved_locale(cfg) {
        "zh" => zh_metric(label),
        "ru" => ru_metric(label),
        _ => label.to_string(),
    }
}

fn zh_metric(label: &str) -> String {
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
        "Reset credit" => "重置额度".into(),
        "Reset credits" => "重置额度".into(),
        "Extra balance" => "额外余额".into(),
        "Kilo Pass" => "Kilo Pass".into(),
        "Requests today" => "今日请求".into(),
        "Requests this month" => "本月请求".into(),
        "Requests this cycle" => "本周期请求".into(),
        "Last used" => "上次使用".into(),
        "Via" => "经由".into(),
        "Sessions" => "会话数".into(),
        other if other.starts_with("Reset credit ") => {
            format!("重置额度 {}", other.trim_start_matches("Reset credit "))
        }
        other if other.ends_with(" weekly") => {
            format!("{} 每周", other.trim_end_matches(" weekly"))
        }
        other => other.to_string(),
    }
}

fn ru_metric(label: &str) -> String {
    match label {
        "Session" => "Сессия".into(),
        "Weekly" => "За неделю".into(),
        "Monthly" => "За месяц".into(),
        "Daily" => "За день".into(),
        "Usage" => "Использование".into(),
        "Credits" => "Кредиты".into(),
        "Credits used" => "Использовано кредитов".into(),
        "API" => "API".into(),
        "Balance" => "Баланс".into(),
        "Vouchers" => "Ваучеры".into(),
        "Cash" => "Наличные".into(),
        "Limit" => "Лимит".into(),
        "Used" => "Использовано".into(),
        "On-demand" => "По факту".into(),
        "Cursor Models" => "Модели Cursor".into(),
        "Other Models" => "Другие модели".into(),
        "Total usage" => "Всего".into(),
        "Bonus" => "Бонус".into(),
        "Extra usage" => "Дополнительно".into(),
        "Extra credits" => "Доп. кредиты".into(),
        "Reset credit" => "Сброс лимита".into(),
        "Reset credits" => "Сброс лимита".into(),
        "Extra balance" => "Доп. баланс".into(),
        "Kilo Pass" => "Kilo Pass".into(),
        "Requests today" => "Запросы сегодня".into(),
        "Requests this month" => "Запросы в этом месяце".into(),
        "Requests this cycle" => "Запросы за цикл".into(),
        "Last used" => "Последнее использование".into(),
        "Via" => "Через".into(),
        "Sessions" => "Сессии".into(),
        other if other.starts_with("Reset credit ") => {
            format!("Сброс лимита {}", other.trim_start_matches("Reset credit "))
        }
        other if other.ends_with(" weekly") => {
            format!("{} за неделю", other.trim_end_matches(" weekly"))
        }
        other => other.to_string(),
    }
}

pub fn pct_left(cfg: &Value, name: &str, label: &str, left: f64) -> String {
    let shown = metric_label(cfg, label);
    match resolved_locale(cfg) {
        "zh" => format!("{name} {shown}: 剩余 {left:.0}%"),
        "ru" => format!("{name} {shown}: осталось {left:.0}%"),
        _ => format!("{name} {shown}: {left:.0}% left"),
    }
}

/// Primary language 0x04 = Chinese (zh-CN, zh-TW, zh-HK, …).
fn langid_is_zh(langid: u16) -> bool {
    const LANG_CHINESE: u16 = 0x04;
    langid & 0x03FF == LANG_CHINESE
}

/// Primary language 0x19 = Russian (ru-RU, ru-MD, …).
fn langid_is_ru(langid: u16) -> bool {
    const LANG_RUSSIAN: u16 = 0x19;
    langid & 0x03FF == LANG_RUSSIAN
}

/// Windows *display* language, not the regional-format locale.
/// Same source the popover asks for via `system_ui_locale`.
pub fn system_ui_locale() -> &'static str {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    let langid = unsafe { GetUserDefaultUILanguage() };
    if langid_is_zh(langid) {
        "zh"
    } else if langid_is_ru(langid) {
        "ru"
    } else {
        "en"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explicit_locale_wins() {
        assert_eq!(resolved_locale(&json!({"locale": "zh"})), "zh");
        assert_eq!(resolved_locale(&json!({"locale": "en"})), "en");
        assert_eq!(resolved_locale(&json!({"locale": "ru"})), "ru");
    }

    #[test]
    fn zh_metric_labels() {
        let zh = json!({"locale": "zh"});
        assert_eq!(metric_label(&zh, "Session"), "会话");
        assert_eq!(metric_label(&zh, "Sonnet weekly"), "Sonnet 每周");
        assert_eq!(metric_label(&zh, "Reset credit 2"), "重置额度 2");
        assert_eq!(metric_label(&json!({"locale": "en"}), "Session"), "Session");
    }

    #[test]
    fn ru_metric_labels() {
        let ru = json!({"locale": "ru"});
        assert_eq!(metric_label(&ru, "Session"), "Сессия");
        assert_eq!(metric_label(&ru, "Sonnet weekly"), "Sonnet за неделю");
        assert_eq!(metric_label(&ru, "Reset credit"), "Сброс лимита");
        assert_eq!(metric_label(&ru, "Reset credit 2"), "Сброс лимита 2");
        assert_eq!(quit_label(&ru), "Выйти из Pane");
    }

    #[test]
    fn chinese_langids_match() {
        assert!(langid_is_zh(0x0804)); // zh-CN
        assert!(langid_is_zh(0x0404)); // zh-TW
        assert!(langid_is_zh(0x0C04)); // zh-HK
        assert!(!langid_is_zh(0x0409)); // en-US
        assert!(!langid_is_zh(0x0411)); // ja
        assert!(!langid_is_zh(0x0419)); // ru-RU
    }

    #[test]
    fn russian_langids_match() {
        assert!(langid_is_ru(0x0419)); // ru-RU
        assert!(!langid_is_ru(0x0409)); // en-US
        assert!(!langid_is_ru(0x0804)); // zh-CN
    }
}

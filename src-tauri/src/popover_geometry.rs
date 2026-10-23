//! Physical-pixel placement shared by popover summon, reset and resize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub fn placement(area: Rect, width: u32, height: u32, saved: Option<(i32, i32)>, locked: bool) -> Rect {
    let w = width.min(area.w);
    let h = height.min(area.h);
    let max_x = area.x.saturating_add((area.w - w) as i32);
    let max_y = area.y.saturating_add((area.h - h) as i32);
    let center = (area.x + ((area.w - w) / 2) as i32, area.y + ((area.h - h) / 2) as i32);
    let (x, y) = if locked { center } else { saved.unwrap_or(center) };
    Rect { x: x.clamp(area.x, max_x), y: y.clamp(area.y, max_y), w, h }
}

pub fn scaled_size(area: Rect, factor: f64) -> (u32, u32) {
    let factor = if factor.is_finite() { factor.clamp(0.8, 1.75) } else { 1.0 };
    // Reference old default: 32.8125% × 75% of work area; new default is
    // 80% of that in each dimension. Leave breathing room at large levels.
    let w = (area.w as f64 * 0.2625 * factor).min(area.w as f64 * 0.96);
    let h = (area.h as f64 * 0.60 * factor).min(area.h as f64 * 0.96);
    (w.round().max(1.0) as u32, h.round().max(1.0) as u32)
}

/// The previous scale used fixed logical pixels. Reset that interpretation
/// once, while preserving every choice saved with the screen-relative basis.
pub fn migrate_scale_settings(obj: &mut serde_json::Map<String, serde_json::Value>) {
    if obj.get("windowScaleVersion").and_then(serde_json::Value::as_u64) != Some(2) {
        obj.insert("windowScaleVersion".into(), serde_json::json!(2));
        obj.insert("windowScale".into(), serde_json::json!(1.0));
        obj.insert("popoverSize".into(), serde_json::Value::Null);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const AREA: Rect = Rect { x: 0, y: 0, w: 2560, h: 1540 };

    #[test]
    fn legacy_scale_migrates_once_and_new_percentages_survive_reload() {
        let mut cfg=serde_json::json!({"windowScale":1.75,"popoverSize":{"w":980,"h":1400},"uiTextSize":"large"});
        migrate_scale_settings(cfg.as_object_mut().unwrap());
        assert_eq!(cfg["windowScale"],1.0);
        assert_eq!(cfg["uiTextSize"],"large");
        assert!(cfg["popoverSize"].is_null());
        cfg["windowScale"]=serde_json::json!(1.17);
        migrate_scale_settings(cfg.as_object_mut().unwrap());
        assert_eq!(cfg["windowScale"],1.17);
    }

    #[test]
    fn screen_relative_scale_uses_smaller_default_and_all_percentages() {
        let reference=Rect {x:0,y:0,w:2560,h:1600};
        assert_eq!(scaled_size(reference,1.0),(672,960));
        assert_eq!(scaled_size(reference,0.8),(538,768));
        assert_eq!(scaled_size(reference,1.47),(988,1411));
        let other=Rect {x:0,y:0,w:1920,h:1080};
        assert_eq!(scaled_size(other,1.0),(504,648));
        let max=scaled_size(reference,1.75);
        assert!(max.0<=reference.w && max.1<=reference.h);
    }

    #[test]
    fn locked_default_ignores_stale_corner_positions() {
        for saved in [None, Some((201,196)), Some((0,0)), Some((2000,1300)), Some((-21333,-21333))] {
            assert_eq!(placement(AREA,700,1000,saved,true), Rect {x:930,y:270,w:700,h:1000});
        }
    }

    #[test]
    fn oversized_scale_fits_work_area_and_remains_centered() {
        assert_eq!(placement(AREA,1225,1750,None,true),Rect {x:667,y:0,w:1225,h:1540});
    }

    #[test]
    fn unlocked_manual_position_is_preserved_but_clamped_on_screen_change() {
        assert_eq!(placement(AREA,700,1000,Some((120,90)),false),Rect {x:120,y:90,w:700,h:1000});
        assert_eq!(placement(AREA,700,1000,Some((2400,1500)),false),Rect {x:1860,y:540,w:700,h:1000});
    }

    #[test]
    fn offset_monitor_and_taskbar_work_area_are_respected() {
        let area=Rect {x:-1920,y:40,w:1920,h:1000};
        assert_eq!(placement(area,560,800,None,true),Rect {x:-1240,y:140,w:560,h:800});
    }

    #[test]
    fn repeated_scale_cycles_keep_default_center_within_rounding_pixel() {
        for size in [(700,1000),(875,1250),(1050,1500),(1225,1750),(700,1000)] {
            let rect=placement(AREA,size.0,size.1,Some((201,196)),true);
            assert!((2*rect.x+rect.w as i32-2560).abs()<=1);
            assert!((2*rect.y+rect.h as i32-1540).abs()<=1);
        }
    }
}

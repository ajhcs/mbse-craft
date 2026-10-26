//! Bounded app-owned envelope; native model and presentation never enter settings.
use crate::docking::{Panel, WorkspaceDock};
use craft_ui::docking::Layout;
use eframe::egui;
use serde::{Deserialize, Serialize};
pub const MAX_LAYOUT_BYTES: usize = 16 * 1024;
pub const MAX_PERSPECTIVES: usize = 8;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    layout: Layout<Panel>,
}
pub fn encode(layout: &Layout<Panel>) -> Result<String, String> {
    WorkspaceDock::check(layout)?;
    let text = serde_json::to_string(&Envelope {
        version: 1,
        layout: layout.clone(),
    })
    .map_err(|e| format!("Workspace encode: {e}"))?;
    if text.len() > MAX_LAYOUT_BYTES {
        return Err("Workspace layout exceeds byte budget".into());
    }
    Ok(text)
}
pub fn decode(text: &str) -> Result<Layout<Panel>, String> {
    if text.is_empty() {
        return Ok(WorkspaceDock::default().layout);
    }
    if text.len() > MAX_LAYOUT_BYTES {
        return Err("Workspace layout exceeds byte budget; original retained".into());
    }
    let saved: Envelope = serde_json::from_str(text)
        .map_err(|e| format!("Workspace layout refused: {e}; original retained"))?;
    if saved.version != 1 {
        return Err("Workspace layout version unsupported; original retained".into());
    }
    WorkspaceDock::check(&saved.layout)?;
    Ok(saved.layout)
}
pub fn name(text: &str) -> Result<&str, String> {
    let text = text.trim();
    if text.is_empty() || text.len() > 64 || text.chars().any(char::is_control) {
        return Err("Use a perspective name of 1..64 bytes without control characters".into());
    }
    Ok(text)
}
pub fn reachable(layout: &Layout<Panel>, screen: egui::Rect) -> Result<Layout<Panel>, String> {
    WorkspaceDock::check(layout)?;
    if !screen.is_finite() || screen.width() <= 0. || screen.height() <= 0. {
        return Err("Workspace viewport unavailable".into());
    }
    let mut next = layout.clone();
    for f in &mut next.floating {
        let w = f.rect[2].max(160.).min(screen.width());
        let h = f.rect[3].max(120.).min(screen.height());
        f.rect = [
            f.rect[0].clamp(screen.left(), screen.right() - w),
            f.rect[1].clamp(screen.top(), screen.bottom() - h),
            w,
            h,
        ];
    }
    WorkspaceDock::check(&next)?;
    Ok(next)
}
#[cfg(test)]
mod tests {
    use super::*;
    use craft_ui::docking::Action;
    #[test]
    fn versioned_roundtrip_keeps_float_tabs_and_explicit_default() {
        let mut d = WorkspaceDock::default();
        d.apply(
            0,
            vec![Action::Float {
                panel: Panel::Inspector,
                rect: [30., 50., 400., 600.],
            }],
        )
        .unwrap();
        assert_eq!(decode(&encode(&d.layout).unwrap()).unwrap(), d.layout);
        assert_eq!(decode("").unwrap(), WorkspaceDock::default().layout);
    }
    #[test]
    fn corrupt_future_missing_duplicate_and_overbudget_are_refused() {
        for s in [
            "{",
            "{\"version\":99,\"layout\":{\"root\":null,\"floating\":[]}}",
            "{\"version\":1,\"layout\":{\"root\":null,\"floating\":[]}}",
        ] {
            assert!(decode(s).is_err());
        }
        let valid = encode(&WorkspaceDock::default().layout).unwrap();
        assert!(decode(&valid.replace("\"inspector\"", "\"browser\"")).is_err());
        assert!(decode(&" ".repeat(MAX_LAYOUT_BYTES + 1)).is_err());
        assert!(name("").is_err());
        assert!(name("a\nb").is_err());
        assert!(name(&"x".repeat(65)).is_err());
        assert_eq!(name(" Review ").unwrap(), "Review");
    }
    #[test]
    fn offscreen_and_shrunken_viewports_reveal_all_floats_without_native_data() {
        let mut d = WorkspaceDock::default();
        d.apply(
            0,
            vec![Action::Float {
                panel: Panel::Inspector,
                rect: [90000., -90000., 9000., 9000.],
            }],
        )
        .unwrap();
        let screen = egui::Rect::from_min_max(egui::pos2(8., 147.), egui::pos2(900., 700.));
        let safe = reachable(&d.layout, screen).unwrap();
        let f = &safe.floating[0];
        assert_eq!(f.rect, [8., 147., 892., 553.]);
        assert_eq!(safe.panels().len(), 3);
        assert_eq!(reachable(&safe, screen).unwrap(), safe);
        assert!(reachable(&safe, egui::Rect::NOTHING).is_err());
        assert_eq!(d.layout.floating[0].rect[0], 90000.);
    }
}

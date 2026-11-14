//! Draw › Pens without a pointer: a whole ink stroke from a list of points, for scripts and the
//! CLI (the same ink shape a drawn stroke makes).

use serde_json::Value;

use super::*;
use crate::tools::ToolKind;
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "draw.stroke",
        "Draw Ink",
        ["Draw", "Pens"],
        None,
        "{points: [[x, y, pressure?]] (slide pt; at least 2), tool?: pen|highlighter, color?: #RRGGBB (pen black; highlighter yellow), width?: pt (pen 1.5; highlighter 10)} → {ink: shape id}",
        has_slide,
        stroke
    )]
}

fn stroke(s: &mut Session, p: &Value) -> Result<Value> {
    let points: Vec<(f64, f64, f32)> = p
        .get("points")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .take(100_000)
                .filter_map(|pt| {
                    let (x, y) = (pt.get(0)?.as_f64()?, pt.get(1)?.as_f64()?);
                    let pressure = pt.get(2).and_then(Value::as_f64).unwrap_or(0.5).clamp(0.0, 1.0) as f32;
                    (x.is_finite() && y.is_finite()).then_some((x, y, pressure))
                })
                .collect()
        })
        .unwrap_or_default();
    if points.len() < 2 {
        return Err(bad("draw.stroke", "`points` needs at least two [x, y] points"));
    }
    let highlighter = match str_param(p, "tool").unwrap_or("pen") {
        "pen" => false,
        "highlighter" => true,
        other => return Err(bad("draw.stroke", format!("unknown tool `{other}`; use pen or highlighter"))),
    };
    let color = match str_param(p, "color") {
        Some(c) => deckcraft_color::Rgba::from_hex(c).ok_or_else(|| bad("draw.stroke", "`color` is not #RRGGBB"))?,
        None if highlighter => deckcraft_color::Rgba::rgb(255, 230, 0),
        None => deckcraft_color::Rgba::rgb(20, 20, 20),
    };
    let width = f64_param(p, "width").filter(|w| w.is_finite() && *w > 0.0).unwrap_or(if highlighter { 10.0 } else { 1.5 }).min(200.0);
    // The stroke goes through the same path as a drawn one; the tool the user had stays.
    let before = s.tool.kind.clone();
    s.tool.kind = ToolKind::Ink { mode: if highlighter { "highlighter" } else { "pen" }.into(), color, width };
    let r = s.add_ink(points);
    s.tool.kind = before;
    r
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::unreachable)]

    use deckcraft_model::ShapeKind;
    use serde_json::json;

    use crate::Session;

    #[test]
    fn a_stroke_joins_the_slide_ink() {
        let mut s = Session::with_new();
        s.execute("draw.stroke", &json!({"points": [[10, 10], [100, 50], [200, 20]]})).unwrap();
        s.execute("draw.stroke", &json!({"points": [[10, 300], [400, 300]], "tool": "highlighter", "color": "#00FF00"})).unwrap();
        let ink: Vec<_> = s.doc().unwrap().current_slide().unwrap().shapes.iter().filter(|x| matches!(x.kind, ShapeKind::Ink { .. })).collect();
        assert_eq!(ink.len(), 1);
        let ShapeKind::Ink { strokes } = &ink[0].kind else { unreachable!() };
        assert_eq!(strokes.len(), 2);
        assert!(!strokes[0].highlighter && strokes[1].highlighter);
        assert!(s.execute("draw.stroke", &json!({"points": [[1, 1]]})).is_err());
    }
}

//! Formatting a title or body placeholder in Slide Master view restyles the slides, as in
//! PowerPoint: the formatting moves from the placeholder's sample text into the master's text
//! styles (`p:txStyles`: `titleStyle`, `bodyStyle`, per level) or, on a layout, into the
//! placeholder's list style (`a:lstStyle`), which every slide on that master or layout inherits.
//! The master's date, footer and slide number placeholders keep theirs in their own list style;
//! a layout's copy of a property the master just changed is dropped, so the layout and its slides
//! inherit the new value (see `release`).

use std::sync::Arc;

use deckcraft_model::text::{LevelStyle, ListStyle, Paragraph, RunProps, TextBody};
use deckcraft_model::{PhType, Presentation, Shape, defaults};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Target;

/// The run formatting a placeholder's sample text sets on purpose (not its language or links,
/// which belong to the text, not to the style).
fn style_part(r: &RunProps) -> RunProps {
    let mut s = r.clone();
    s.lang = None;
    s.link = None;
    s.hover = None;
    s.no_proof = None;
    s
}

fn has_style(r: &RunProps) -> bool {
    *r != RunProps { lang: r.lang.clone(), link: r.link.clone(), hover: r.hover.clone(), no_proof: r.no_proof, ..Default::default() }
}

/// Move the explicit formatting of `body`'s paragraphs into `style` (by paragraph level) and
/// clear it from the text. True when anything moved.
fn promote(body: &mut TextBody, style: &mut ListStyle) -> bool {
    let mut changed = false;
    for p in &mut body.paragraphs {
        let Paragraph { level, props, runs, def_props, end_props } = p;
        // An empty paragraph (the master's date and footer samples are `<a:p/>`) carries the
        // formatting only on its end mark, where Format ▸ Font Color on the whole shape puts it.
        let run = runs
            .iter()
            .map(|r| style_part(&r.props))
            .find(has_style)
            .or_else(|| Some(style_part(end_props)).filter(has_style))
            .or_else(|| Some(style_part(def_props)).filter(has_style));
        let para_set = *props != Default::default();
        if run.is_none() && !para_set {
            continue;
        }
        let old = style.level(*level).cloned().unwrap_or_default();
        let mut new = LevelStyle { para: props.clone(), run: run.unwrap_or_default() };
        new.para.inherit(&old.para);
        new.run.inherit(&old.run);
        style.set(*level, new);
        *props = Default::default();
        for r in runs.iter_mut().map(|r| &mut r.props).chain([def_props, end_props]) {
            *r = RunProps { lang: r.lang.take(), link: r.link.take(), hover: r.hover.take(), no_proof: r.no_proof, ..Default::default() };
        }
        changed = true;
    }
    changed
}

fn is_title(sh: &Shape) -> bool {
    sh.ph_type().is_some_and(|k| k.is_title())
}

fn is_body(sh: &Shape) -> bool {
    matches!(sh.ph_type(), Some(PhType::Body | PhType::Obj))
}

fn is_footer(kind: Option<PhType>) -> bool {
    matches!(kind, Some(PhType::Date | PhType::Footer | PhType::SlideNum | PhType::Header))
}

/// Drop from `layout` each property the master changed (`old` → `new`) where the layout only
/// repeats the master's old value or the default a new presentation gives (`default`), so it
/// inherits the new value. A value the layout set to something else stays: it overrides the
/// master, as in PowerPoint.
fn release_props<T: Serialize + DeserializeOwned>(layout: &mut T, old: &T, new: &T, default: &T) -> bool {
    let obj = |v: &T| match serde_json::to_value(v) {
        Ok(Value::Object(m)) => m,
        _ => Default::default(),
    };
    let (mut l, o, n, d) = (obj(layout), obj(old), obj(new), obj(default));
    let mut changed = false;
    for (k, nv) in &n {
        let ov = o.get(k).unwrap_or(&Value::Null);
        if nv == ov {
            continue;
        }
        if let Some(lv) = l.get(k)
            && !lv.is_null()
            && (lv == ov || lv == nv || d.get(k) == Some(lv))
        {
            l.remove(k);
            changed = true;
        }
    }
    if changed && let Ok(v) = serde_json::from_value(Value::Object(l)) {
        *layout = v;
        return true;
    }
    false
}

/// `release_props` level by level for a layout placeholder's list style; a level left with
/// nothing set is removed.
fn release(layout: &mut ListStyle, old: &ListStyle, new: &ListStyle, default: &ListStyle) -> bool {
    let mut changed = false;
    for (i, slot) in layout.levels.iter_mut().enumerate() {
        let lvl = i as u8;
        let Some(l) = slot.as_mut() else { continue };
        let (o, n, d) = (
            old.level(lvl).cloned().unwrap_or_default(),
            new.level(lvl).cloned().unwrap_or_default(),
            default.level(lvl).cloned().unwrap_or_default(),
        );
        changed |= release_props(&mut l.para, &o.para, &n.para, &d.para);
        changed |= release_props(&mut l.run, &o.run, &n.run, &d.run);
        if *l == LevelStyle::default() {
            *slot = None;
        }
    }
    changed
}

/// Promote placeholder formatting in the master or layout `target` shows. True when the
/// presentation changed.
pub fn promote_placeholder_formatting(doc: &mut Presentation, target: Target) -> bool {
    match target {
        Target::Master { master } => {
            let Some(m) = doc.masters.get_mut(master) else { return false };
            let m = Arc::make_mut(m);
            let mut changed = false;
            let mut footers = vec![];
            let (title_style, body_style) = (&mut m.title_style, &mut m.body_style);
            for sh in m.shapes.iter_mut() {
                let kind = sh.ph_type();
                if is_footer(kind) {
                    if let (Some(kind), Some(t)) = (kind, sh.text.as_mut()) {
                        let old = t.list_style.clone();
                        let mut style = std::mem::take(&mut t.list_style);
                        if promote(t, &mut style) {
                            changed = true;
                            footers.push((kind, old, style.clone()));
                        }
                        t.list_style = style;
                    }
                    continue;
                }
                let style = if is_title(sh) {
                    &mut *title_style
                } else if is_body(sh) {
                    &mut *body_style
                } else {
                    continue;
                };
                if let Some(t) = sh.text.as_mut() {
                    changed |= promote(t, style);
                }
            }
            // Slides take a footer's formatting from their layout's placeholder of the same kind
            // first; a layout made by DeckCraft or PowerPoint repeats the master's 12 pt `tx1`
            // 75% there, which would otherwise hide the master's new value from every slide.
            for (kind, old, new) in &footers {
                let default = defaults::footer_list_style(*kind);
                for l in &mut m.layouts {
                    for sh in l.shapes.iter_mut().filter(|s| s.ph_type() == Some(*kind)) {
                        if let Some(t) = sh.text.as_mut() {
                            release(&mut t.list_style, old, new, &default);
                        }
                    }
                }
            }
            changed
        }
        Target::Layout { master, layout } => {
            let Some(l) = doc.masters.get_mut(master).map(Arc::make_mut).and_then(|m| m.layouts.get_mut(layout)) else { return false };
            let mut changed = false;
            for sh in l.shapes.iter_mut().filter(|s| is_title(s) || is_body(s) || is_footer(s.ph_type())) {
                if let Some(t) = sh.text.as_mut() {
                    let mut style = std::mem::take(&mut t.list_style);
                    changed |= promote(t, &mut style);
                    t.list_style = style;
                }
            }
            changed
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use crate::Session;

    #[test]
    fn bold_on_the_master_title_restyles_every_slide() {
        let mut s = Session::with_new();
        s.execute("view.slideMaster", &json!({})).unwrap();
        let title = s.doc().unwrap().shapes().iter().find(|x| x.ph_type().is_some_and(|k| k.is_title())).unwrap().id;
        s.execute("edit.select", &json!({"ids": [title.0]})).unwrap();
        s.execute("format.bold", &json!({})).unwrap();
        let m = &s.doc().unwrap().doc.masters[0];
        assert_eq!(m.title_style.level(0).and_then(|l| l.run.bold), Some(true), "titleStyle level 1 is bold");
        let sample = m.shapes.iter().find(|x| x.id == title).unwrap();
        assert!(sample.text.as_ref().unwrap().paragraphs.iter().flat_map(|p| &p.runs).all(|r| r.props.bold.is_none()), "not on the sample text");
    }

    #[test]
    fn font_color_on_the_master_footers_reaches_the_slides_and_the_saved_file() {
        use deckcraft_color::Rgba;
        use deckcraft_model::resolve::{self, Ctx};
        use deckcraft_model::{Fill, PhType};

        let mut s = Session::with_new();
        s.execute("design.headerFooter", &json!({"date": true, "slideNumber": true, "footer": true, "footerText": "Confidential"})).unwrap();
        s.execute("view.slideMaster", &json!({})).unwrap();
        let kinds = [PhType::Date, PhType::Footer, PhType::SlideNum];
        let ids: Vec<u32> = s.doc().unwrap().shapes().iter().filter(|x| x.ph_type().is_some_and(|k| kinds.contains(&k))).map(|x| x.id.0).collect();
        assert_eq!(ids.len(), 3);
        s.execute("edit.select", &json!({"ids": ids})).unwrap();
        s.execute("format.color", &json!({"color": "#595959"})).unwrap();
        s.execute("view.closeMaster", &json!({})).unwrap();

        let grey = Rgba::rgb(0x59, 0x59, 0x59);
        let check = |p: &deckcraft_model::Presentation, when: &str| {
            let m = &p.masters[0];
            for k in kinds {
                let sh = m.shapes.iter().find(|x| x.ph_type() == Some(k)).unwrap();
                let lvl = sh.text.as_ref().unwrap().list_style.level(0).unwrap();
                assert!(matches!(&lvl.run.fill, Some(Fill::Solid { .. })), "{when}: the master's {k:?} list style has the colour");
                assert_eq!(lvl.run.size, Some(12.0), "{when}: the size stays");
                for l in &m.layouts {
                    for sh in l.shapes.iter().filter(|x| x.ph_type() == Some(k)) {
                        let fill = sh.text.as_ref().and_then(|t| t.list_style.level(0)).and_then(|l| l.run.fill.clone());
                        assert!(fill.is_none(), "{when}: layout {} {k:?} inherits the colour", l.name);
                    }
                }
            }
            let slide = &p.slides[0];
            let ctx = Ctx::for_slide(p, slide).unwrap();
            let mut seen = 0;
            for sh in slide.shapes.iter().filter(|x| x.ph_type().is_some_and(|k| kinds.contains(&k))) {
                let t = sh.text.as_ref().unwrap();
                let para = &t.paragraphs[0];
                let r = resolve::run(&ctx, sh, para, &para.runs[0].props);
                let Some(Fill::Solid { color }) = r.fill else { panic!("{when}: solid") };
                assert_eq!(ctx.color(&color, None), grey, "{when}: the slide's {:?} is drawn in #595959", sh.ph_type());
                seen += 1;
            }
            assert!(seen >= 2, "{when}: the slide has footers ({seen})");
        };
        let doc = s.doc().unwrap().doc.clone();
        check(&doc, "after the edit");
        let back = deckcraft_pptx::import(&deckcraft_pptx::export(&doc).unwrap()).unwrap();
        check(&back, "after saving and opening");
    }
}

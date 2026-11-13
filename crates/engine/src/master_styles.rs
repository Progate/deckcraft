//! Formatting a title or body placeholder in Slide Master view restyles the slides, as in
//! PowerPoint: the formatting moves from the placeholder's sample text into the master's text
//! styles (`p:txStyles`: `titleStyle`, `bodyStyle`, per level) or, on a layout, into the
//! placeholder's list style (`a:lstStyle`), which every slide on that master or layout inherits.

use std::sync::Arc;

use deckcraft_model::text::{LevelStyle, ListStyle, Paragraph, RunProps, TextBody};
use deckcraft_model::{PhType, Presentation, Shape};

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
        let Paragraph { level, props, runs, .. } = p;
        let run = runs.iter().map(|r| style_part(&r.props)).find(has_style);
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
        for r in runs.iter_mut() {
            r.props = RunProps {
                lang: r.props.lang.take(),
                link: r.props.link.take(),
                hover: r.props.hover.take(),
                no_proof: r.props.no_proof,
                ..Default::default()
            };
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

/// Promote placeholder formatting in the master or layout `target` shows. True when the
/// presentation changed.
pub fn promote_placeholder_formatting(doc: &mut Presentation, target: Target) -> bool {
    match target {
        Target::Master { master } => {
            let Some(m) = doc.masters.get_mut(master) else { return false };
            let m = Arc::make_mut(m);
            let mut changed = false;
            let (title_style, body_style) = (&mut m.title_style, &mut m.body_style);
            for sh in m.shapes.iter_mut() {
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
            changed
        }
        Target::Layout { master, layout } => {
            let Some(l) = doc.masters.get_mut(master).map(Arc::make_mut).and_then(|m| m.layouts.get_mut(layout)) else { return false };
            let mut changed = false;
            for sh in l.shapes.iter_mut().filter(|s| is_title(s) || is_body(s)) {
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
}

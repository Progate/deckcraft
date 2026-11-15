//! Lengths shown in the interface: inches in English, centimetres in Japanese (as PowerPoint's
//! Japanese version shows sizes and positions). The presentation keeps points; only what is shown
//! and typed changes.

/// Do lengths show in centimetres (the interface is Japanese)?
pub fn metric() -> bool {
    crate::i18n::current().code() == "ja"
}

/// Points in one displayed unit (72 per inch, 72 / 2.54 per centimetre).
pub fn pt_per_unit() -> f64 {
    if metric() { 72.0 / 2.54 } else { 72.0 }
}

/// The unit's suffix in a field (`"` or ` cm`).
pub fn suffix() -> &'static str {
    if metric() { " cm" } else { "\"" }
}

/// The unit's name in a label (`in` or `cm`).
pub fn name() -> &'static str {
    if metric() { "cm" } else { "in" }
}

/// Points → the displayed unit.
pub fn from_pt(pt: f64) -> f64 {
    pt / pt_per_unit()
}

/// The displayed unit → points.
pub fn to_pt(v: f64) -> f64 {
    v * pt_per_unit()
}

/// The spinner step: 0.1 inch or 0.1 cm.
pub fn step() -> f64 {
    0.1
}

/// A label with its unit: `Width (in):` becomes `幅 (cm):` in Japanese (the catalog has the
/// `(cm)` wording under the same English key).
pub fn label(english_in: &str) -> String {
    let shown = crate::i18n::t(english_in);
    if metric() { shown.replace("(in)", "(cm)").replace("(インチ)", "(cm)") } else { shown.to_string() }
}

/// Text for a field showing `pt` in the displayed unit (two decimals, trailing zeros dropped).
pub fn fmt_pt(pt: f64) -> String {
    crate::widgets::fmt_num(from_pt(pt), "")
}

/// A typed length in the displayed unit (a trailing `cm`, `"` or `in` is allowed) → points.
pub fn parse_pt(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches("cm").trim_end_matches("in").trim_end_matches('"').trim();
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(to_pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_uses_inches() {
        assert_eq!(suffix(), "\"");
        assert!((to_pt(1.0) - 72.0).abs() < 1e-9);
        assert_eq!(parse_pt("2\""), Some(144.0));
    }

    #[test]
    fn japanese_uses_centimetres() {
        crate::i18n::set_current(crate::i18n::Lang::from_code("ja").unwrap_or(crate::i18n::Lang::EN));
        assert_eq!(suffix(), " cm");
        assert!((to_pt(2.54) - 72.0).abs() < 1e-9);
        assert_eq!(fmt_pt(72.0), "2.54");
        assert!(parse_pt("2.54 cm").is_some_and(|p| (p - 72.0).abs() < 1e-9));
        crate::i18n::set_current(crate::i18n::Lang::EN);
    }
}

//! Interface languages. Strings in the code stay English and are the lookup keys; a catalog per
//! language (`*.tsv`, format in `ja.tsv`) maps them to display text when the UI is drawn.
//! Command ids, document text, file names, the control channel, the CLI and MCP never see
//! translated text, so scripts and agents are unaffected. A string without a translation shows in
//! English, so coverage can grow one entry at a time.
//!
//! The language follows the environment: `DECKCRAFT_LANGUAGE` first, then the POSIX locale
//! variables in their order of precedence (`LC_ALL`, `LC_MESSAGES`, `LANG`). A locale without a
//! catalog, `C` and `POSIX` mean English.
//!
//! This is the system WordCraft uses (`wordcraft/crates/ui-egui/src/i18n`, catalog parser copied
//! from it), so the catalog format and habits carry over between the Crafting Apps.
//! The Japanese catalog and its terminology table are documented in `docs/localization-ja.md`.
//!
//! # Looking strings up
//! - [`tl!`](crate::tl) / [`t`]: a string in the current language. [`tr`]: in a given one.
//! - [`tc`]: worded for a context (`ribbon`, `effect`), falling back to the plain entry.
//! - [`fmt`]: fill `{name}` placeholders after a lookup; translators may reorder them.

mod catalog;

use std::cell::Cell;
use std::sync::OnceLock;

use catalog::Catalog;

/// One supported interface language.
pub struct LangInfo {
    /// BCP 47 code, lowercase (`ja`).
    pub code: &'static str,
    /// The language's name in itself.
    pub name: &'static str,
    /// Catalog file contents (empty for the built-in English).
    pub source: &'static str,
    catalog: OnceLock<Catalog>,
}

/// The registry. English first: it is the fallback and the source language.
pub static LANGUAGES: [LangInfo; 2] = [
    LangInfo { code: "en", name: "English", source: "", catalog: OnceLock::new() },
    LangInfo { code: "ja", name: "日本語", source: include_str!("ja.tsv"), catalog: OnceLock::new() },
];

impl LangInfo {
    fn catalog(&self) -> &Catalog {
        self.catalog.get_or_init(|| {
            let (catalog, errors) = Catalog::parse(self.source);
            for error in errors {
                log::warn!("{} interface catalog: {error}; that entry shows in English", self.code);
            }
            catalog
        })
    }
}

/// A language the UI can be shown in (a handle into [`LANGUAGES`]).
#[derive(Clone, Copy)]
pub struct Lang(&'static LangInfo);

impl std::fmt::Debug for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lang({})", self.0.code)
    }
}

impl PartialEq for Lang {
    fn eq(&self, other: &Self) -> bool {
        self.0.code == other.0.code
    }
}

impl Eq for Lang {}

impl Lang {
    pub const EN: Lang = Lang(&LANGUAGES[0]);

    pub fn code(self) -> &'static str {
        self.0.code
    }

    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// The catalog's text, for the interface fonts to look through once (so every glyph the
    /// translated interface draws has a face before the first frame that needs it).
    pub fn catalog_text(self) -> &'static str {
        self.0.source
    }

    /// A language by its exact code (any case).
    pub fn from_code(code: &str) -> Option<Lang> {
        LANGUAGES.iter().find(|l| l.code.eq_ignore_ascii_case(code)).map(Lang)
    }

    /// Every registered language.
    pub fn all() -> impl Iterator<Item = Lang> {
        LANGUAGES.iter().map(Lang)
    }

    fn catalog(self) -> &'static Catalog {
        self.0.catalog()
    }
}

/// Candidate codes for a locale tag, most specific first: `ja_JP.UTF-8` → `ja-jp`, `ja`.
fn candidates(tag: &str) -> Vec<String> {
    let base = tag.split(['.', '@']).next().unwrap_or("").replace('_', "-").to_ascii_lowercase();
    let parts: Vec<&str> = base.split('-').filter(|p| !p.is_empty()).collect();
    (1..=parts.len()).rev().map(|n| parts.get(..n).unwrap_or_default().join("-")).collect()
}

/// The registered language for a locale tag such as `ja_JP.UTF-8`; `None` when there is no
/// catalog for it. `C`/`POSIX` mean English.
pub fn lang_from_tag(tag: &str) -> Option<Lang> {
    let cands = candidates(tag);
    if matches!(cands.first().map(String::as_str), Some("c" | "posix")) {
        return Some(Lang::EN);
    }
    cands.iter().find_map(|c| Lang::from_code(c))
}

/// The language for a set of environment variables (`get` reads one by name): `DECKCRAFT_LANGUAGE`,
/// then the first of `LC_ALL`, `LC_MESSAGES`, `LANG` that is set and non-empty, as POSIX picks the
/// messages locale. English when none names a language with a catalog.
pub fn lang_from_env(get: impl Fn(&str) -> Option<String>) -> Lang {
    if let Some(l) = get("DECKCRAFT_LANGUAGE").filter(|v| !v.is_empty()).and_then(|v| lang_from_tag(&v)) {
        return l;
    }
    ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|name| get(name).filter(|v| !v.is_empty())).and_then(|v| lang_from_tag(&v)).unwrap_or(Lang::EN)
}

/// The system language (looked up once). English when it can't be determined or isn't supported.
pub fn system_lang() -> Lang {
    // Tests drive the UI by its English labels whatever the machine's language is.
    if cfg!(test) {
        return Lang::EN;
    }
    static SYSTEM: OnceLock<Lang> = OnceLock::new();
    *SYSTEM.get_or_init(|| lang_from_env(|name| std::env::var(name).ok()))
}

thread_local! {
    // Per thread: parallel test harnesses must not change each other's language.
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// Set the language the UI is drawn in (the app does this every frame).
pub fn set_current(lang: Lang) {
    CURRENT.set(lang);
}

/// The language the UI is drawn in.
pub fn current() -> Lang {
    CURRENT.get()
}

/// Does `lang` have a catalog entry for this string? (English never does: it is the source.)
pub fn has(lang: Lang, s: &str) -> bool {
    lang.catalog().plain(s).is_some()
}

/// `s` in the current language ([`tr`] with [`current`]).
pub fn t(s: &str) -> &str {
    tr(current(), s)
}

/// `s` in `lang`; strings without a translation come back unchanged.
pub fn tr(lang: Lang, s: &str) -> &str {
    lang.catalog().plain(s).unwrap_or(s)
}

/// `s` in the current language as worded for `context` (one of `catalog::CONTEXTS`), falling
/// back to the plain translation and then to English.
pub fn tc<'a>(context: &str, s: &'a str) -> &'a str {
    current().catalog().in_context(context, s).unwrap_or(s)
}

/// A string the interface builds at run time (`format!`), translated when the whole string has
/// an entry: effect and layout names from the model, labels chosen from a table.
pub fn t_owned(s: String) -> String {
    match current().catalog().plain(&s) {
        Some(x) => x.to_string(),
        None => s,
    }
}

/// A label with an action after a fixed English prefix, e.g. `Undo Typing`: both parts translated
/// where the catalog has them (`Undo {action}`).
pub fn prefixed(prefix: &str, rest: &str) -> String {
    if rest.is_empty() {
        return t(prefix).to_string();
    }
    fmt(t(&format!("{prefix} {{action}}")), &[("action", t(rest))])
}

/// Fill `{name}` placeholders in one pass. Unknown placeholders stay as written, and inserted
/// values are never read as templates again, so a file name containing `{n}` stays intact.
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once('{') {
        out.push_str(before);
        let Some((name, after_close)) = after_open.split_once('}') else {
            out.push('{');
            out.push_str(after_open);
            return out;
        };
        match args.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = after_close;
    }
    out.push_str(rest);
    out
}

/// The text shown in an empty placeholder (`Click to add title`) in the current language. The
/// renderer calls it through [`deckcraft_render::set_prompt_text`], so slides drawn for the
/// editor read like the rest of the interface; files never contain it.
pub fn prompt(s: &'static str) -> &'static str {
    tr(system_lang(), s)
}

#[cfg(test)]
mod tests;

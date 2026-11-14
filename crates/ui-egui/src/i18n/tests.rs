#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| vars.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
}

#[test]
fn locale_variables_pick_the_language() {
    assert_eq!(lang_from_env(env(&[("LANG", "ja_JP.UTF-8")])).code(), "ja");
    assert_eq!(lang_from_env(env(&[])).code(), "en");
    assert_eq!(lang_from_env(env(&[("LANG", "fr_FR.UTF-8")])).code(), "en", "no catalog: English");
    assert_eq!(lang_from_env(env(&[("LANG", "C")])).code(), "en");
    // POSIX precedence: LC_ALL, then LC_MESSAGES, then LANG.
    assert_eq!(lang_from_env(env(&[("LC_ALL", "C"), ("LANG", "ja_JP.UTF-8")])).code(), "en");
    assert_eq!(lang_from_env(env(&[("LC_MESSAGES", "ja_JP"), ("LANG", "en_US.UTF-8")])).code(), "ja");
    assert_eq!(lang_from_env(env(&[("LC_ALL", ""), ("LANG", "ja")])).code(), "ja", "empty variables are skipped");
    // DECKCRAFT_LANGUAGE wins over the locale.
    assert_eq!(lang_from_env(env(&[("DECKCRAFT_LANGUAGE", "en"), ("LANG", "ja_JP.UTF-8")])).code(), "en");
    assert_eq!(lang_from_env(env(&[("DECKCRAFT_LANGUAGE", "ja"), ("LANG", "en_US.UTF-8")])).code(), "ja");
}

#[test]
fn the_japanese_catalog_has_no_errors() {
    let ja = Lang::from_code("ja").unwrap();
    let (_, errors) = catalog::Catalog::parse(ja.catalog_text());
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn japanese_uses_powerpoint_terms() {
    let ja = Lang::from_code("ja").unwrap();
    for (en, want) in [
        ("Home", "ホーム"),
        ("Insert", "挿入"),
        ("Draw", "描画"),
        ("Design", "デザイン"),
        ("Transitions", "画面切り替え"),
        ("Animations", "アニメーション"),
        ("Slide Show", "スライド ショー"),
        ("Review", "校閲"),
        ("View", "表示"),
        ("New\nSlide", "新しい\nスライド"),
        ("Click to add title", "タイトルを入力"),
        ("Click to add text", "テキストを入力"),
        ("Title Slide", "タイトル スライド"),
        ("Title and Content", "タイトルとコンテンツ"),
        ("Fade", "フェード"),
    ] {
        assert_eq!(tr(ja, en), want, "{en:?}");
    }
}

#[test]
fn untranslated_strings_and_english_stay_as_written() {
    assert_eq!(tr(Lang::EN, "Home"), "Home");
    let ja = Lang::from_code("ja").unwrap();
    assert_eq!(tr(ja, "no such string 12345"), "no such string 12345");
}

#[test]
fn placeholders_fill_in_any_order() {
    assert_eq!(fmt("{a} of {b}", &[("b", "2"), ("a", "1")]), "1 of 2");
    assert_eq!(fmt("{a} {missing}", &[("a", "x")]), "x {missing}");
}

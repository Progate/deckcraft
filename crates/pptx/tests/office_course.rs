//! What the Progate PowerPoint course checks in saved files: chart elements, media playback
//! options, Mark as Final, the password to modify and Inspect Document. Each feature is made with
//! the same commands the UI runs, saved as .pptx, checked as PowerPoint XML and read back.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Read;

use deckcraft_engine::{DocLock, Session};
use deckcraft_model::{KeptPart, ShapeKind};
use serde_json::json;

fn part(bytes: &[u8], name: &str) -> String {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
    let mut out = String::new();
    z.by_name(name).unwrap_or_else(|_| panic!("no part {name}")).read_to_string(&mut out).expect("utf8");
    out
}

fn has_part(bytes: &[u8], name: &str) -> bool {
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip").by_name(name).is_ok()
}

fn save(s: &Session) -> Vec<u8> {
    deckcraft_pptx::export(&s.doc().unwrap().doc).expect("export")
}

/// The first chart part's XML.
fn chart_xml(bytes: &[u8]) -> String {
    let z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
    let name = z.file_names().find(|n| n.starts_with("ppt/charts/chart") && n.ends_with(".xml")).expect("a chart part").to_string();
    part(bytes, &name)
}

fn first_chart(p: &deckcraft_model::Presentation) -> deckcraft_model::chart::Chart {
    let mut found = None;
    deckcraft_model::walk(&p.slides[0].shapes, &mut |s, _| {
        if let ShapeKind::Chart(c) = &s.kind {
            found.get_or_insert_with(|| (**c).clone());
        }
    });
    found.expect("a chart")
}

#[test]
fn chart_elements_are_saved_as_chart_xml_and_read_back() {
    let mut s = Session::with_new();
    s.execute("insert.chart", &json!({"type": "column"})).unwrap();
    s.execute(
        "chart.options",
        &json!({"title": "売上", "legend": "b", "dataLabels": true, "gridlines": true, "minorGridlines": true, "xAxisTitle": "月", "yAxisTitle": "金額"}),
    )
    .unwrap();
    let bytes = save(&s);
    let xml = chart_xml(&bytes);
    assert!(xml.contains("<c:autoTitleDeleted val=\"0\"/>"), "{xml}");
    assert!(xml.contains("売上") && xml.contains("月") && xml.contains("金額"));
    assert!(xml.contains("<c:legendPos val=\"b\"/>"));
    assert!(xml.contains("<c:dLbls>"));
    assert!(xml.contains("<c:majorGridlines/>") && xml.contains("<c:minorGridlines"));
    let c = first_chart(&deckcraft_pptx::import(&bytes).unwrap());
    assert_eq!(c.title.as_deref(), Some("売上"));
    assert_eq!(c.legend.as_deref(), Some("b"));
    assert!(c.data_labels && c.gridlines && c.minor_gridlines);
    assert_eq!(c.axis_titles, (Some("月".into()), Some("金額".into())));

    // Turning elements off: no title (autoTitleDeleted), no legend, no axis titles.
    s.execute("chart.options", &json!({"title": null, "legend": null, "xAxisTitle": null, "yAxisTitle": null, "dataLabels": false})).unwrap();
    let xml = chart_xml(&save(&s));
    assert!(xml.contains("<c:autoTitleDeleted val=\"1\"/>"));
    assert!(!xml.contains("<c:legend>"));
    let c = first_chart(&deckcraft_pptx::import(&save(&s)).unwrap());
    assert_eq!((c.title, c.legend, c.axis_titles, c.data_labels), (None, None, (None, None), false));
}

#[test]
fn quick_layouts_set_their_elements() {
    let mut s = Session::with_new();
    s.execute("insert.chart", &json!({"type": "column"})).unwrap();
    s.execute("chart.quickLayout", &json!({"layout": 5})).unwrap();
    let c = first_chart(&deckcraft_pptx::import(&save(&s)).unwrap());
    assert!(c.title.is_some() && c.legend.is_none() && c.gridlines);
    assert!(c.axis_titles.0.is_some() && c.axis_titles.1.is_some());
    s.execute("chart.quickLayout", &json!({"layout": 2})).unwrap();
    let c = first_chart(&deckcraft_pptx::import(&save(&s)).unwrap());
    assert_eq!(c.legend.as_deref(), Some("t"));
    assert!(c.data_labels && !c.gridlines && c.axis_titles == (None, None));
    assert!(s.execute("chart.quickLayout", &json!({"layout": 99})).is_err());
}

fn media_session(video: bool) -> Session {
    let mut s = Session::with_new();
    let (cmd, name, data) = if video {
        ("insert.video", "clip.mp4", deckcraft_engine::cmd::base64_encode(b"\x00\x00\x00\x18ftypmp42not-a-real-video"))
    } else {
        ("insert.audio", "sound.wav", deckcraft_engine::cmd::base64_encode(b"RIFF\x24\x00\x00\x00WAVEfmt not-real"))
    };
    s.execute(cmd, &json!({"name": name, "data": data})).unwrap();
    s
}

fn first_clip(p: &deckcraft_model::Presentation) -> deckcraft_model::MediaClip {
    let mut found = None;
    deckcraft_model::walk(&p.slides[0].shapes, &mut |s, _| {
        if let ShapeKind::Media(m) = &s.kind {
            found.get_or_insert_with(|| m.clone());
        }
    });
    found.expect("a clip")
}

#[test]
fn media_playback_options_are_saved_in_the_timing_tree() {
    let mut s = media_session(true);
    s.execute(
        "media.options",
        &json!({"autoplay": true, "loop": true, "rewind": true, "acrossSlides": true, "hide": true, "fullScreen": true, "volume": 0.5}),
    )
    .unwrap();
    let bytes = save(&s);
    let xml = part(&bytes, "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:video fullScrn=\"1\"><p:cMediaNode vol=\"50000\" numSld=\"999\" showWhenStopped=\"0\">"), "{xml}");
    assert!(xml.contains("repeatCount=\"indefinite\" fill=\"remove\" display=\"0\""), "{xml}");
    // Start: Automatically is a Play effect before the first click.
    assert!(xml.contains("presetClass=\"mediacall\"") && xml.contains("nodeType=\"afterEffect\"") && xml.contains("cmd=\"playFrom(0.0)\""), "{xml}");
    let q = deckcraft_pptx::import(&bytes).unwrap();
    let m = first_clip(&q);
    assert!(m.autoplay && m.loop_play && m.rewind && m.play_across_slides && m.hide_while_not_playing && m.full_screen);
    assert!((m.volume - 0.5).abs() < 1e-9);
    assert!(q.slides[0].animations.is_empty(), "Start: Automatically comes back as the clip's option, not as an animation");
    // A second save writes the Play effect once.
    let again = deckcraft_pptx::export(&q).unwrap();
    assert_eq!(part(&again, "ppt/slides/slide1.xml").matches("playFrom(0.0)").count(), 1);
}

#[test]
fn defaults_and_a_muted_audio_clip() {
    let mut s = media_session(false);
    let xml = part(&save(&s), "ppt/slides/slide1.xml");
    assert!(xml.contains("<p:audio><p:cMediaNode vol=\"100000\"><p:cTn id=\"2\" fill=\"hold\" display=\"0\">"), "{xml}");
    assert!(!xml.contains("mediacall"), "On Click (the default) adds no effect");
    s.execute("media.options", &json!({"volume": 0.0})).unwrap();
    let bytes = save(&s);
    assert!(part(&bytes, "ppt/slides/slide1.xml").contains("<p:cMediaNode vol=\"0\" mute=\"1\">"));
    let m = first_clip(&deckcraft_pptx::import(&bytes).unwrap());
    assert_eq!(m.volume, 0.0);
    assert!(!m.autoplay && !m.loop_play && !m.rewind && !m.hide_while_not_playing);
}

#[test]
fn mark_as_final_is_a_custom_property() {
    let mut s = Session::with_new();
    s.execute("file.markFinal", &json!({})).unwrap();
    let bytes = save(&s);
    let custom = part(&bytes, "docProps/custom.xml");
    assert!(custom.contains("name=\"_MarkAsFinal\"") && custom.contains(">true</vt:bool>"), "{custom}");
    assert!(part(&bytes, "_rels/.rels").contains("relationships/custom-properties\" Target=\"docProps/custom.xml\""));
    assert!(part(&bytes, "[Content_Types].xml").contains("custom-properties+xml"));
    let q = deckcraft_pptx::import(&bytes).unwrap();
    assert!(q.props.mark_as_final);
    assert!(q.kept_parts.is_empty(), "the property was the part's only content");
    assert_eq!(DocLock::on_open(&q), Some(DocLock::Final));

    // Edit Anyway takes the mark off; the saved file has no custom properties left.
    s.execute("file.editAnyway", &json!({})).unwrap();
    assert!(!has_part(&save(&s), "docProps/custom.xml"));
}

#[test]
fn mark_as_final_joins_existing_custom_properties() {
    let mut p = deckcraft_model::defaults::new_presentation(None);
    p.kept_parts.push(KeptPart {
        name: "docProps/custom.xml".into(),
        content_type: "application/vnd.openxmlformats-officedocument.custom-properties+xml".into(),
        xml: r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="2" name="Dept"><vt:lpwstr>Sales</vt:lpwstr></property></Properties>"#
            .into(),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties".into(),
        from_root: true,
    });
    p.props.mark_as_final = true;
    let bytes = deckcraft_pptx::export(&p).unwrap();
    let custom = part(&bytes, "docProps/custom.xml");
    assert!(custom.contains("name=\"Dept\"") && custom.contains("pid=\"3\" name=\"_MarkAsFinal\""), "{custom}");
    let q = deckcraft_pptx::import(&bytes).unwrap();
    assert!(q.props.mark_as_final);
    assert_eq!(q.kept_parts.len(), 1);
    assert!(q.kept_parts[0].xml.contains("Dept") && !q.kept_parts[0].xml.contains("_MarkAsFinal"));
    // Unmarked, the other property stays.
    let mut q = q;
    q.props.mark_as_final = false;
    let custom = part(&deckcraft_pptx::export(&q).unwrap(), "docProps/custom.xml");
    assert!(custom.contains("Dept") && !custom.contains("_MarkAsFinal"));
}

#[test]
fn the_password_to_modify_is_a_modify_verifier() {
    let mut s = Session::with_new();
    s.execute("file.setModifyPassword", &json!({"password": "progate"})).unwrap();
    let bytes = save(&s);
    let xml = part(&bytes, "ppt/presentation.xml");
    let at = xml.find("<p:modifyVerifier ").expect("modifyVerifier");
    let tag = &xml[at..at + xml[at..].find("/>").unwrap()];
    for attr in [
        "cryptProviderType=\"rsaAES\"",
        "cryptAlgorithmClass=\"hash\"",
        "cryptAlgorithmType=\"typeAny\"",
        "cryptAlgorithmSid=\"14\"",
        "spinCount=\"100000\"",
        "saltData=\"",
        "hashData=\"",
    ] {
        assert!(tag.contains(attr), "{tag}");
    }
    assert!(xml.find("</p:defaultTextStyle>").unwrap() < at, "after defaultTextStyle");
    assert!(!xml.contains("password") && !xml.contains("progate"), "never the password itself");
    let q = deckcraft_pptx::import(&bytes).unwrap();
    let v = q.modify_verifier.clone().expect("verifier read back");
    assert_eq!(Some(&v), s.doc().unwrap().doc.modify_verifier.as_ref());
    assert!(deckcraft_engine::cmd::protect::verify("progate", &v));
    assert!(!deckcraft_engine::cmd::protect::verify("Progate", &v));
    assert_eq!(DocLock::on_open(&q), Some(DocLock::Password));

    // Removing the password removes the element.
    s.execute("file.setModifyPassword", &json!({"password": ""})).unwrap();
    assert!(!part(&save(&s), "ppt/presentation.xml").contains("modifyVerifier"));
}

#[test]
fn the_standard_algorithm_name_form_is_read_too() {
    let mut s = Session::with_new();
    s.execute("file.setModifyPassword", &json!({"password": "x"})).unwrap();
    let bytes = save(&s);
    let xml = part(&bytes, "ppt/presentation.xml");
    let v = s.doc().unwrap().doc.modify_verifier.clone().unwrap();
    let at = xml.find("<p:modifyVerifier ").unwrap();
    let end = at + xml[at..].find("/>").unwrap() + 2;
    let other = format!(
        "{}<p:modifyVerifier algorithmName=\"SHA-512\" hashValue=\"{}\" saltValue=\"{}\" spinValue=\"{}\"/>{}",
        &xml[..at],
        v.hash,
        v.salt,
        v.spin_count,
        &xml[end..]
    );
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes[..])).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        out.start_file(name.clone(), zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut out, if name == "ppt/presentation.xml" { other.as_bytes() } else { &data }).unwrap();
    }
    let rewritten = out.finish().unwrap().into_inner();
    let q = deckcraft_pptx::import(&rewritten).unwrap();
    assert_eq!(q.modify_verifier, Some(v));
}

#[test]
fn document_inspector_removals_are_saved() {
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({})).unwrap();
    s.execute("slide.notes", &json!({"text": "話すこと"})).unwrap();
    s.execute("comment.add", &json!({"text": "要確認", "author": "Taro"})).unwrap();
    s.execute("file.properties", &json!({"author": "Taro", "company": "Progate"})).unwrap();
    s.execute("shape.insert", &json!({"preset": "rect", "rect": [-400, 20, 100, 100]})).unwrap();
    s.execute("slide.new", &json!({})).unwrap();
    s.execute("slide.hide", &json!({})).unwrap();
    let before = save(&s);
    assert!(has_part(&before, "ppt/notesSlides/notesSlide2.xml") || has_part(&before, "ppt/notesSlides/notesSlide1.xml"));
    let v = s.execute("file.inspect", &json!({})).unwrap();
    assert_eq!(
        (v["comments"].as_u64(), v["notes"].as_u64(), v["hiddenSlides"].as_u64(), v["offSlide"].as_u64()),
        (Some(1), Some(1), Some(1), Some(1))
    );
    for kind in deckcraft_engine::cmd::protect::INSPECT_KINDS {
        s.execute("file.inspectRemove", &json!({"kind": kind})).unwrap();
    }
    let bytes = save(&s);
    let q = deckcraft_pptx::import(&bytes).unwrap();
    assert_eq!(q.slides.len(), 2, "the hidden slide is gone");
    assert!(q.slides.iter().all(|sl| sl.comments.is_empty() && sl.notes.is_empty() && !sl.hidden));
    assert!(q.props.author.is_empty() && q.props.company.is_empty());
    let core = part(&bytes, "docProps/core.xml");
    assert!(!core.contains("Taro"), "{core}");
    assert!(!part(&bytes, "docProps/app.xml").contains("Progate"));
    let z = zip::ZipArchive::new(std::io::Cursor::new(&bytes[..])).unwrap();
    assert!(!z.file_names().any(|n| n.starts_with("ppt/notesSlides/") || n.starts_with("ppt/comments")), "no notes or comment parts");
    let slide2 = part(&bytes, "ppt/slides/slide2.xml");
    assert!(!slide2.contains("<a:off x=\"-"), "the off-slide shape is gone");
}

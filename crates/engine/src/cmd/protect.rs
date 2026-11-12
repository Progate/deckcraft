//! File › Info: Check for Issues › Inspect Document, and Protect Presentation (Mark as Final and
//! the password to modify, set in PowerPoint from Save As › Tools › General Options).

use std::sync::Arc;

use deckcraft_model::{ModifyVerifier, Presentation, Shape};
use serde_json::{Value, json};
use sha2::{Digest, Sha512};

use super::*;
use crate::{DocLock, EngineError, Result, Session};

/// Why an edit was refused while the presentation is marked final.
pub const FINAL_MESSAGE: &str = "This presentation is marked as final. Choose Edit Anyway to edit it.";
/// Why an edit was refused while the password to modify has not been given.
pub const PASSWORD_MESSAGE: &str = "This presentation is protected by a password to modify. Enter the password or open it read-only.";
/// The answer to a wrong password.
pub const WRONG_PASSWORD: &str = "The password you supplied is not correct.";

/// Iterations of the password hash (PowerPoint's default).
const SPIN_COUNT: u32 = 100_000;

/// What Inspect Document looks for, in the order it lists them.
pub const INSPECT_KINDS: [&str; 5] = ["comments", "properties", "hiddenSlides", "offSlide", "notes"];

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "file.inspect",
            "Inspect Document",
            ["File", "Info"],
            None,
            "{} → {comments, properties: [field], hiddenSlides, offSlide, notes} (counts of what Remove All would take out)",
            has_doc,
            inspect
        ),
        cmd!(
            "file.inspectRemove",
            "Remove All",
            ["File", "Info"],
            None,
            "{kind: comments|properties|hiddenSlides|offSlide|notes} → {removed}",
            has_doc,
            inspect_remove
        ),
        cmd!(noundo "file.markFinal", "Mark as Final", ["File", "Info"], None, "{on?: bool (true)}", has_doc, mark_final),
        cmd!(noundo "file.editAnyway", "Edit Anyway", ["File", "Info"], None, "{}", has_doc, edit_anyway),
        cmd!(
            noundo "file.setModifyPassword",
            "Password to Modify",
            ["File", "Save As", "General Options"],
            None,
            "{password: string (empty removes it)}",
            has_doc,
            set_modify_password
        ),
        cmd!(
            noundo "file.unlock",
            "Password",
            ["File"],
            None,
            "{password: string} | {readOnly: true} (a presentation opened with a password to modify)",
            has_doc,
            unlock
        ),
        cmd!(query "file.protection", "Protection", ["File", "Info"], None, "{} → {final, modifyPassword, lock: final|password|readOnly|null}", has_doc, protection),
    ]
}

/// A shape lying wholly outside the slide (`w` × `h` points): Off-Slide Content.
fn off_slide(sh: &Shape, w: f64, h: f64) -> bool {
    let Some(x) = sh.xfrm.as_ref() else { return false };
    x.x + x.w.abs() <= 0.0 || x.y + x.h.abs() <= 0.0 || x.x >= w || x.y >= h
}

/// The document property fields that hold something (personal information).
fn filled_properties(doc: &Presentation) -> Vec<&'static str> {
    let p = &doc.props;
    [
        ("title", &p.title),
        ("subject", &p.subject),
        ("author", &p.author),
        ("keywords", &p.keywords),
        ("comments", &p.comments),
        ("category", &p.category),
        ("company", &p.company),
        ("lastModifiedBy", &p.last_modified_by),
    ]
    .into_iter()
    .filter(|(_, v)| !v.trim().is_empty())
    .map(|(k, _)| k)
    .collect()
}

fn comment_count(doc: &Presentation) -> usize {
    doc.slides.iter().map(|s| s.comments.len()).sum()
}

fn notes_count(doc: &Presentation) -> usize {
    doc.slides.iter().filter(|s| !s.notes.is_empty()).count()
}

fn off_slide_count(doc: &Presentation) -> usize {
    let (w, h) = (doc.slide_size.width, doc.slide_size.height);
    doc.slides.iter().map(|s| s.shapes.iter().filter(|sh| off_slide(sh, w, h)).count()).sum()
}

fn inspect(s: &mut Session, _p: &Value) -> Result<Value> {
    let doc = &s.doc()?.doc;
    Ok(json!({
        "comments": comment_count(doc),
        "properties": filled_properties(doc),
        "hiddenSlides": doc.slides.iter().filter(|sl| sl.hidden).count(),
        "offSlide": off_slide_count(doc),
        "notes": notes_count(doc),
    }))
}

fn inspect_remove(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = str_param(p, "kind").ok_or_else(|| bad("file.inspectRemove", "missing `kind`"))?;
    let removed = match kind {
        "comments" => s.edit(|doc, _| {
            let n = comment_count(doc);
            for sl in &mut doc.slides {
                if !sl.comments.is_empty() {
                    Arc::make_mut(sl).comments.clear();
                }
            }
            Ok(n)
        })?,
        "notes" => s.edit(|doc, _| {
            let n = notes_count(doc);
            for sl in &mut doc.slides {
                if !sl.notes.is_empty() {
                    Arc::make_mut(sl).notes = Default::default();
                }
            }
            Ok(n)
        })?,
        "properties" => s.edit(|doc, _| {
            let n = filled_properties(doc).len();
            let pr = &mut doc.props;
            for f in [
                &mut pr.title,
                &mut pr.subject,
                &mut pr.author,
                &mut pr.keywords,
                &mut pr.comments,
                &mut pr.category,
                &mut pr.company,
                &mut pr.last_modified_by,
            ] {
                f.clear();
            }
            // Comment authors are personal information too.
            for sl in &mut doc.slides {
                if sl.comments.iter().any(|c| !c.author.is_empty()) {
                    let sl = Arc::make_mut(sl);
                    let mut stack: Vec<&mut deckcraft_model::Comment> = sl.comments.iter_mut().collect();
                    while let Some(c) = stack.pop() {
                        c.author.clear();
                        stack.extend(c.replies.iter_mut());
                    }
                }
            }
            Ok(n)
        })?,
        "offSlide" => s.edit(|doc, sel| {
            let (w, h) = (doc.slide_size.width, doc.slide_size.height);
            let mut n = 0;
            for sl in &mut doc.slides {
                let gone: Vec<_> = sl.shapes.iter().filter(|sh| off_slide(sh, w, h)).map(|sh| sh.id).collect();
                if gone.is_empty() {
                    continue;
                }
                n += gone.len();
                let sl = Arc::make_mut(sl);
                sl.shapes.retain(|sh| !gone.contains(&sh.id));
                sl.animations.retain(|a| !gone.contains(&a.shape) && a.trigger.is_none_or(|t| !gone.contains(&t)));
            }
            sel.shapes.clear();
            Ok(n)
        })?,
        "hiddenSlides" => {
            let ids: Vec<u32> = s.doc()?.doc.slides.iter().filter(|sl| sl.hidden).map(|sl| sl.id.0).collect();
            if ids.is_empty() {
                0
            } else {
                s.execute("slide.delete", &json!({"ids": ids}))?;
                ids.len()
            }
        }
        other => return Err(bad("file.inspectRemove", format!("unknown kind `{other}`; use one of {}", INSPECT_KINDS.join(", ")))),
    };
    Ok(json!({"removed": removed}))
}

fn mark_final(s: &mut Session, p: &Value) -> Result<Value> {
    let on = bool_or(p, "on", true);
    let st = s.doc_mut()?;
    // Marking final (or taking it off) is allowed on a final presentation; it is not an edit of
    // its content, so it goes past the lock that `Session::edit` keeps.
    let mut doc = (*st.doc).clone();
    doc.props.mark_as_final = on;
    st.doc = Arc::new(doc);
    st.revision += 1;
    if on {
        st.lock = Some(DocLock::Final);
    } else if st.lock == Some(DocLock::Final) {
        st.lock = None;
    }
    Ok(json!({"final": on}))
}

fn edit_anyway(s: &mut Session, _p: &Value) -> Result<Value> {
    let st = s.doc_mut()?;
    if st.lock == Some(DocLock::Final) {
        st.lock = None;
    }
    // As in PowerPoint, editing a final presentation takes the mark off.
    if st.doc.props.mark_as_final {
        let mut doc = (*st.doc).clone();
        doc.props.mark_as_final = false;
        st.doc = Arc::new(doc);
        st.revision += 1;
    }
    ok()
}

/// 16 bytes of salt from the process's hash keys (seeded from the operating system's random
/// source) and the clock.
fn salt() -> [u8; 16] {
    use std::hash::{BuildHasher, Hasher};
    let mut out = [0u8; 16];
    for (i, chunk) in out.chunks_mut(8).enumerate() {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_usize(i);
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        if let Ok(d) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            h.write_u128(d.as_nanos());
        }
        chunk.copy_from_slice(&h.finish().to_le_bytes());
    }
    out
}

/// The password hash of ECMA-376 (Part 1, the `modifyVerifier` and `documentProtection` hash):
/// H0 = SHA-512(salt ‖ password as UTF-16LE), then H(n) = SHA-512(H(n−1) ‖ n as 32-bit little
/// endian), `spin` times.
pub fn password_hash(password: &str, salt: &[u8], spin: u32) -> Vec<u8> {
    let mut h = Sha512::new();
    h.update(salt);
    for u in password.encode_utf16() {
        h.update(u.to_le_bytes());
    }
    let mut cur = h.finalize().to_vec();
    for i in 0..spin {
        let mut h = Sha512::new();
        h.update(&cur);
        h.update(i.to_le_bytes());
        cur = h.finalize().to_vec();
    }
    cur
}

/// A new verifier for `password` (SHA-512, random salt, PowerPoint's iteration count).
pub fn verifier_for(password: &str) -> ModifyVerifier {
    let salt = salt();
    let hash = password_hash(password, &salt, SPIN_COUNT);
    ModifyVerifier { algorithm: "SHA-512".into(), hash: base64_encode(&hash), salt: base64_encode(&salt), spin_count: SPIN_COUNT }
}

/// Does `password` match `v`? Only SHA-512 verifiers can be checked; others never match.
pub fn verify(password: &str, v: &ModifyVerifier) -> bool {
    if !v.algorithm.eq_ignore_ascii_case("SHA-512") {
        return false;
    }
    let (Some(salt), Some(want)) = (base64_decode(&v.salt), base64_decode(&v.hash)) else { return false };
    password_hash(password, &salt, v.spin_count.min(10_000_000)) == want
}

fn set_modify_password(s: &mut Session, p: &Value) -> Result<Value> {
    let password = str_param(p, "password").ok_or_else(|| bad("file.setModifyPassword", "missing `password`"))?;
    if password.chars().count() > 255 {
        return Err(bad("file.setModifyPassword", "a password has at most 255 characters"));
    }
    let v = (!password.is_empty()).then(|| verifier_for(password));
    let on = v.is_some();
    s.edit(|doc, _| {
        doc.modify_verifier = v;
        Ok(())
    })?;
    Ok(json!({"modifyPassword": on}))
}

fn unlock(s: &mut Session, p: &Value) -> Result<Value> {
    let st = s.doc_mut()?;
    if bool_or(p, "readOnly", false) {
        if st.lock == Some(DocLock::Password) {
            st.lock = Some(DocLock::ReadOnly);
        }
        return Ok(json!({"lock": "readOnly"}));
    }
    let password = str_param(p, "password").ok_or_else(|| bad("file.unlock", "pass `password` or `readOnly: true`"))?;
    let Some(v) = st.doc.modify_verifier.clone() else {
        if st.lock == Some(DocLock::Password) {
            st.lock = None;
        }
        return ok();
    };
    if !verify(password, &v) {
        return Err(EngineError::Other(WRONG_PASSWORD.into()));
    }
    if matches!(st.lock, Some(DocLock::Password | DocLock::ReadOnly)) {
        // A final presentation stays final after its password: PowerPoint asks for both.
        st.lock = st.doc.props.mark_as_final.then_some(DocLock::Final);
    }
    Ok(json!({"lock": lock_name(st.lock)}))
}

fn lock_name(l: Option<DocLock>) -> Value {
    match l {
        Some(DocLock::Final) => json!("final"),
        Some(DocLock::Password) => json!("password"),
        Some(DocLock::ReadOnly) => json!("readOnly"),
        None => Value::Null,
    }
}

fn protection(s: &mut Session, _p: &Value) -> Result<Value> {
    let st = s.doc()?;
    Ok(json!({"final": st.doc.props.mark_as_final, "modifyPassword": st.doc.modify_verifier.is_some(), "lock": lock_name(st.lock)}))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use serde_json::json;

    use super::*;

    #[test]
    fn the_hash_follows_the_standard_iteration() {
        // Zero iterations is SHA-512(salt ‖ UTF-16LE password).
        let h0 = password_hash("a", b"", 0);
        let mut d = Sha512::new();
        d.update([b'a', 0]);
        assert_eq!(h0, d.finalize().to_vec());
        let h1 = password_hash("a", b"", 1);
        let mut d = Sha512::new();
        d.update(&h0);
        d.update(0u32.to_le_bytes());
        assert_eq!(h1, d.finalize().to_vec());
    }

    #[test]
    fn a_password_verifies_and_a_wrong_one_does_not() {
        let v = verifier_for("secret");
        assert_eq!(v.algorithm, "SHA-512");
        assert_eq!(v.spin_count, SPIN_COUNT);
        assert!(verify("secret", &v));
        assert!(!verify("Secret", &v));
        assert_ne!(verifier_for("secret").salt, v.salt, "each verifier has its own salt");
    }

    #[test]
    fn a_final_presentation_refuses_edits_until_edit_anyway() {
        let mut s = Session::with_new();
        s.execute("file.markFinal", &json!({})).unwrap();
        assert!(s.execute("slide.new", &json!({})).is_err());
        s.execute("file.editAnyway", &json!({})).unwrap();
        assert!(!s.doc().unwrap().doc.props.mark_as_final);
        s.execute("slide.new", &json!({})).unwrap();
    }

    #[test]
    fn a_password_lock_opens_with_the_password_or_read_only() {
        let mut s = Session::with_new();
        s.execute("file.setModifyPassword", &json!({"password": "pw"})).unwrap();
        s.doc_mut().unwrap().lock = Some(DocLock::Password);
        assert!(s.execute("slide.new", &json!({})).is_err());
        assert!(s.execute("file.unlock", &json!({"password": "nope"})).is_err());
        s.execute("file.unlock", &json!({"readOnly": true})).unwrap();
        assert_eq!(s.doc().unwrap().lock, Some(DocLock::ReadOnly));
        s.execute("slide.new", &json!({})).unwrap();
        s.execute("file.unlock", &json!({"password": "pw"})).unwrap();
        assert_eq!(s.doc().unwrap().lock, None);
    }

    #[test]
    fn inspect_finds_and_removes_each_kind() {
        let mut s = Session::with_new();
        s.execute("slide.new", &json!({})).unwrap();
        s.execute("slide.new", &json!({})).unwrap();
        s.execute("slide.notes", &json!({"text": "speaker"})).unwrap();
        s.execute("file.properties", &json!({"author": "Hanako", "title": "Deck"})).unwrap();
        s.execute("shape.insert", &json!({"preset": "rect", "rect": [2000, 2000, 50, 50]})).unwrap();
        s.execute("slide.hide", &json!({"index": 1})).unwrap();
        let v = s.execute("file.inspect", &json!({})).unwrap();
        assert_eq!(v["notes"], 1);
        assert_eq!(v["hiddenSlides"], 1);
        assert_eq!(v["offSlide"], 1);
        assert_eq!(v["properties"], json!(["title", "author"]));
        for kind in INSPECT_KINDS {
            s.execute("file.inspectRemove", &json!({"kind": kind})).unwrap();
        }
        let v = s.execute("file.inspect", &json!({})).unwrap();
        assert_eq!((v["notes"].clone(), v["hiddenSlides"].clone(), v["offSlide"].clone()), (json!(0), json!(0), json!(0)));
        assert_eq!(v["properties"], json!([]));
        assert_eq!(s.doc().unwrap().doc.slides.len(), 2, "the hidden slide went");
    }
}

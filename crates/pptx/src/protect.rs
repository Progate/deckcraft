//! File › Info › Protect Presentation: Mark as Final (the `_MarkAsFinal` custom document property
//! in `docProps/custom.xml`) and the password to modify (`p:modifyVerifier` in
//! `ppt/presentation.xml`).
//!
//! Custom properties are otherwise kept verbatim ([`deckcraft_model::KeptPart`]); `_MarkAsFinal`
//! is taken out of the kept part on import and written back from
//! [`deckcraft_model::Properties::mark_as_final`], so the flag has one source of truth.

use deckcraft_model::{KeptPart, ModifyVerifier};

use crate::xml::{A, El, W};

pub const REL_CUSTOM_PROPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties";
pub const CT_CUSTOM_PROPS: &str = "application/vnd.openxmlformats-officedocument.custom-properties+xml";
const NS_CUSTOM: &str = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties";
const NS_VT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes";
/// The format id every user-defined custom property carries.
const FMTID_USER: &str = "{D5CDD505-2E9C-101B-9397-08002B2CF9AE}";
const MARK_AS_FINAL: &str = "_MarkAsFinal";

fn is_custom_props(k: &KeptPart) -> bool {
    k.rel_type == REL_CUSTOM_PROPS && k.from_root
}

/// The byte range of the `property` element named `_MarkAsFinal` in `xml`, if any.
fn mark_range(xml: &str) -> Option<(usize, usize)> {
    let at = xml.find(&format!("name=\"{MARK_AS_FINAL}\"")).or_else(|| xml.find(&format!("name='{MARK_AS_FINAL}'")))?;
    let start = xml.get(..at)?.rfind('<')?;
    let rest = xml.get(at..)?;
    let end = at + rest.find("property>")? + "property>".len();
    Some((start, end))
}

/// Is `_MarkAsFinal` set to true in this custom properties XML?
fn marked_final(xml: &str) -> bool {
    let Ok(doc) = crate::xml::parse(xml.as_bytes()) else { return false };
    doc.root.children_named("property").any(|p| p.attr("name") == Some(MARK_AS_FINAL) && property_true(p))
}

fn property_true(p: &El) -> bool {
    p.elements().next().is_some_and(|v| matches!(v.text().trim(), "true" | "1"))
}

/// Read Mark as Final from the kept parts and take the property out of them (dropping a custom
/// properties part that held nothing else). Returns whether the presentation is marked final.
pub fn take_mark_as_final(kept: &mut Vec<KeptPart>) -> bool {
    let mut marked = false;
    let mut drop = None;
    for (i, k) in kept.iter_mut().enumerate().filter(|(_, k)| is_custom_props(k)) {
        marked |= marked_final(&k.xml);
        if let Some((a, b)) = mark_range(&k.xml) {
            k.xml.replace_range(a..b, "");
        }
        let empty = crate::xml::parse(k.xml.as_bytes()).map(|d| d.root.children_named("property").next().is_none()).unwrap_or(false);
        if empty {
            drop = Some(i);
        }
    }
    if let Some(i) = drop {
        kept.remove(i);
    }
    marked
}

/// The kept parts to write, with `_MarkAsFinal` added to the custom properties when `final_`
/// (creating `docProps/custom.xml` when the file had none).
pub fn with_mark_as_final(kept: &[KeptPart], final_: bool) -> Vec<KeptPart> {
    let mut out = kept.to_vec();
    if !final_ {
        return out;
    }
    if let Some(k) = out.iter_mut().find(|k| is_custom_props(k)) {
        let pid = next_pid(&k.xml);
        let prop = property_xml(pid);
        if let Some(close) = k.xml.rfind("</") {
            k.xml.insert_str(close, &prop);
            return out;
        }
    }
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Properties xmlns=\"{NS_CUSTOM}\" xmlns:vt=\"{NS_VT}\">{}</Properties>",
        property_xml(2)
    );
    out.push(KeptPart {
        name: "docProps/custom.xml".into(),
        content_type: CT_CUSTOM_PROPS.into(),
        xml,
        rel_type: REL_CUSTOM_PROPS.into(),
        from_root: true,
    });
    out
}

/// The next free `pid` (custom properties number from 2).
fn next_pid(xml: &str) -> u32 {
    let Ok(doc) = crate::xml::parse(xml.as_bytes()) else { return 2 };
    doc.root.children_named("property").filter_map(|p| p.u32("pid")).max().map_or(2, |m| m.saturating_add(1).max(2))
}

fn property_xml(pid: u32) -> String {
    // `vt` is declared on the value itself: a kept part may bind the prefix to nothing or another name.
    format!("<property fmtid=\"{FMTID_USER}\" pid=\"{pid}\" name=\"{MARK_AS_FINAL}\"><vt:bool xmlns:vt=\"{NS_VT}\">true</vt:bool></property>")
}

/// The `cryptAlgorithmSid` of a hash algorithm (ECMA-376 Part 1, 19.2.1.19 / 22.4.2.x).
fn algorithm_sid(name: &str) -> Option<u32> {
    match name.to_ascii_uppercase().as_str() {
        "MD2" => Some(1),
        "MD4" => Some(2),
        "MD5" => Some(3),
        "SHA-1" | "SHA1" => Some(4),
        "SHA-256" | "SHA256" => Some(12),
        "SHA-384" | "SHA384" => Some(13),
        "SHA-512" | "SHA512" => Some(14),
        _ => None,
    }
}

fn algorithm_name(sid: u32) -> Option<&'static str> {
    match sid {
        1 => Some("MD2"),
        2 => Some("MD4"),
        3 => Some("MD5"),
        4 => Some("SHA-1"),
        12 => Some("SHA-256"),
        13 => Some("SHA-384"),
        14 => Some("SHA-512"),
        _ => None,
    }
}

/// `p:modifyVerifier`, read in either form the standard allows: `algorithmName`/`hashValue`/
/// `saltValue`/`spinValue`, or the `cryptAlgorithmSid`/`hashData`/`saltData`/`spinCount` form
/// PowerPoint writes.
pub fn read_modify_verifier(e: &El) -> Option<ModifyVerifier> {
    let algorithm = e.attr("algorithmName").map(String::from).or_else(|| e.u32("cryptAlgorithmSid").and_then(algorithm_name).map(String::from))?;
    let hash = e.attr("hashValue").or_else(|| e.attr("hashData"))?.to_string();
    let salt = e.attr("saltValue").or_else(|| e.attr("saltData")).unwrap_or("").to_string();
    let spin_count = e.u32("spinValue").or_else(|| e.u32("spinCount")).unwrap_or(0).min(10_000_000);
    (!hash.is_empty()).then_some(ModifyVerifier { algorithm, hash, salt, spin_count })
}

/// Write `p:modifyVerifier` in the form PowerPoint writes (`rsaAES`, `cryptAlgorithmSid`), or
/// with `algorithmName` for an algorithm that has no sid.
pub fn write_modify_verifier(w: &mut W, v: &ModifyVerifier) {
    let a = match algorithm_sid(&v.algorithm) {
        Some(sid) => A::new()
            .a("cryptProviderType", "rsaAES")
            .a("cryptAlgorithmClass", "hash")
            .a("cryptAlgorithmType", "typeAny")
            .a("cryptAlgorithmSid", sid)
            .a("spinCount", v.spin_count)
            .a("saltData", &v.salt)
            .a("hashData", &v.hash),
        None => A::new().a("algorithmName", &v.algorithm).a("hashValue", &v.hash).a("saltValue", &v.salt).a("spinValue", v.spin_count),
    };
    w.empty("p:modifyVerifier", a);
}

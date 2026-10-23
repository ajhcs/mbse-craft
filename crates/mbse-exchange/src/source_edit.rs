//! Narrow, byte-preserving edits to a newly created Capella XML copy.
//! This module has no native-model or execution API.
use crate::v2::{self, ElementIdentity, ElementKey, ProjectionSet, ProjectionStage};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{ops::Range, path::Path};

pub const PROFILE: &str = "mbse-craft.external.xml-edit/1";
const CTX: &str = "http://www.polarsys.org/capella/core/ctx/7.0.0";

/// In-memory proposal. Every public field is rebound against freshly read source
/// before persistence; altered source anchors are refused.
#[derive(Clone, Debug, Serialize)]
pub struct NameProposal {
    pub profile: &'static str,
    pub source_set_digest: String,
    pub primary_sha256: String,
    pub element: ElementKey,
    pub source_namespace: String,
    pub source_classifier: String,
    pub original_value: String,
    pub proposed_value: String,
    pub value_range: Range<usize>,
    pub original_encoded_value: String,
    quote: u8,
}
#[derive(Debug, Serialize)]
pub struct EditReceipt {
    pub profile: &'static str,
    pub original_source_set_digest: String,
    pub output_source_set_digest: String,
    pub original_primary_sha256: String,
    pub output_primary_sha256: String,
    pub changed_value_range: Range<usize>,
    pub original_encoded_bytes: usize,
    pub replacement_encoded_bytes: usize,
    pub outside_patch_bytes_unchanged: bool,
    pub xml_reparsed: bool,
    pub persisted_ids_and_reference_spellings_unchanged: bool,
    pub engineering_validated: bool,
    pub limitations: Vec<String>,
}
fn eligible<'a>(set: &'a ProjectionSet, key: &ElementKey) -> Result<&'a v2::SemanticNode, String> {
    if set.assets.len() != 1 || set.stage != ProjectionStage::PartialSemanticProjection {
        return Err("SOURCE_EDIT_DIRECT_XML_REQUIRED: archives, refused interpretations and companion sets are unsupported; use exact original-byte copy".into());
    }
    let node = set
        .nodes
        .iter()
        .find(|n| &n.key == key)
        .ok_or("SOURCE_EDIT_ELEMENT_NOT_FOUND")?;
    if node.source_type.namespace != CTX
        || !matches!(
            node.source_type.local.as_str(),
            "Mission" | "Capability" | "SystemComponent"
        )
        || !matches!(
            node.kind,
            v2::SemanticKind::CapellaMission
                | v2::SemanticKind::CapellaCapability
                | v2::SemanticKind::CapellaComponent
        )
        || !matches!(node.key.identity, ElementIdentity::Original(_))
        || node
            .attributes
            .iter()
            .any(|a| matches!(a.name.as_str(), "href" | "idref"))
    {
        return Err("SOURCE_EDIT_UNSUPPORTED_CLASS: only identified owned ctx/7.0.0 Mission, Capability and SystemComponent with an existing unqualified name are supported".into());
    }
    Ok(node)
}
fn value_range(bytes: &[u8], position: usize) -> Result<(Range<usize>, u8), String> {
    let mut i = position;
    if bytes.get(i..i + 4) != Some(b"name") {
        return Err("SOURCE_EDIT_ATTRIBUTE_POSITION".into());
    }
    i += 4;
    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    if bytes.get(i) != Some(&b'=') {
        return Err("SOURCE_EDIT_ATTRIBUTE_ASSIGNMENT".into());
    }
    i += 1;
    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let quote = *bytes.get(i).ok_or("SOURCE_EDIT_ATTRIBUTE_QUOTE")?;
    if quote != b'\'' && quote != b'"' {
        return Err("SOURCE_EDIT_ATTRIBUTE_QUOTE".into());
    }
    let start = i + 1;
    let length = bytes[start..]
        .iter()
        .position(|b| *b == quote)
        .ok_or("SOURCE_EDIT_ATTRIBUTE_END")?;
    Ok((start..start + length, quote))
}
// roxmltree validates declaration syntax but does not enforce the declared
// version/encoding values. This write profile accepts XML 1.0 and UTF-8 only.
fn supported_declaration(text: &str) -> Result<(), String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = text.strip_prefix("<?xml") else {
        return Ok(());
    };
    if !rest.as_bytes().first().is_some_and(u8::is_ascii_whitespace) {
        return Ok(()); // An xml-stylesheet PI is not an XML declaration.
    }
    let (declaration, _) = rest.split_once("?>").ok_or("SOURCE_EDIT_XML_DECLARATION")?;
    if declaration.len() > 1024 {
        return Err("SOURCE_EDIT_XML_DECLARATION_LIMIT".into());
    }
    let mut remaining = declaration;
    let mut version = false;
    let mut encoding = false;
    let mut standalone = false;
    while !remaining
        .trim_start_matches([' ', '\t', '\r', '\n'])
        .is_empty()
    {
        remaining = remaining.trim_start_matches([' ', '\t', '\r', '\n']);
        let length = remaining
            .bytes()
            .take_while(u8::is_ascii_alphabetic)
            .count();
        if length == 0 {
            return Err("SOURCE_EDIT_XML_DECLARATION".into());
        }
        let field = &remaining[..length];
        remaining = remaining[length..].trim_start_matches([' ', '\t', '\r', '\n']);
        remaining = remaining
            .strip_prefix('=')
            .ok_or("SOURCE_EDIT_XML_DECLARATION")?
            .trim_start_matches([' ', '\t', '\r', '\n']);
        let quote = *remaining
            .as_bytes()
            .first()
            .ok_or("SOURCE_EDIT_XML_DECLARATION")?;
        if quote != b'\'' && quote != b'"' {
            return Err("SOURCE_EDIT_XML_DECLARATION".into());
        }
        let end = remaining[1..]
            .find(quote as char)
            .ok_or("SOURCE_EDIT_XML_DECLARATION")?
            + 1;
        let value = &remaining[1..end];
        match field {
            "version" if !version && value == "1.0" => version = true,
            "encoding" if version && !encoding && !standalone && value.eq_ignore_ascii_case("UTF-8") => encoding = true,
            "standalone" if version && !standalone && matches!(value, "yes" | "no") => standalone = true,
            _ => return Err("SOURCE_EDIT_XML_1_0_UTF8_REQUIRED: declarations must specify literal version 1.0 and, when present, UTF-8 encoding".into()),
        }
        remaining = &remaining[end + 1..];
    }
    if !version {
        return Err("SOURCE_EDIT_XML_1_0_UTF8_REQUIRED".into());
    }
    Ok(())
}
fn encoded(value: &str, quote: u8) -> Result<String, String> {
    if value.len() > v2::MAX_VALUE_BYTES {
        return Err("SOURCE_EDIT_VALUE_LIMIT".into());
    }
    let mut out = String::new();
    for c in value.chars() {
        // XML 1.0 Char production. Escape whitespace to preserve exact decoded values.
        if !matches!(c, '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
        {
            return Err("SOURCE_EDIT_XML_CHARACTER".into());
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' if quote == b'"' => out.push_str("&quot;"),
            '\'' if quote == b'\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            _ => out.push(c),
        }
    }
    Ok(out)
}
impl NameProposal {
    pub fn prepare(
        set: &ProjectionSet,
        key: &ElementKey,
        proposed: String,
    ) -> Result<Self, String> {
        let node = eligible(set, key)?;
        let text =
            std::str::from_utf8(set.primary_bytes()).map_err(|_| "SOURCE_EDIT_UTF8_REQUIRED")?;
        let doc = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
        supported_declaration(text)?;
        let element = doc
            .descendants()
            .find(|n| n.is_element() && n.range().start == node.location.start)
            .ok_or("SOURCE_EDIT_LOCATOR_CHANGED")?;
        let attribute = element
            .attributes()
            .find(|a| a.namespace().is_none() && a.name() == "name")
            .ok_or("SOURCE_EDIT_EXISTING_NAME_REQUIRED")?;
        let (range, quote) = value_range(text.as_bytes(), attribute.position())?;
        encoded(&proposed, quote)?;
        Ok(Self {
            profile: PROFILE,
            source_set_digest: set.source_set_digest.clone(),
            primary_sha256: set.primary_sha256.clone(),
            element: key.clone(),
            source_namespace: node.source_type.namespace.clone(),
            source_classifier: node.source_type.local.clone(),
            original_value: attribute.value().into(),
            proposed_value: proposed,
            original_encoded_value: text[range.clone()].into(),
            value_range: range,
            quote,
        })
    }
    /// Re-read the complete supplied source, rebind the proposal, then create a new private file.
    /// The caller must retain proposals on error and guard application close while IO is pending.
    pub fn create_new_copy(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<EditReceipt, String> {
        let fresh = v2::open(source)?;
        if fresh.source_set_digest != self.source_set_digest
            || fresh.primary_sha256 != self.primary_sha256
        {
            return Err(
                "SOURCE_EDIT_STALE_SOURCE: reopen changed source and make a new proposal".into(),
            );
        }
        let rebound = Self::prepare(&fresh, &self.element, self.proposed_value.clone())?;
        if serde_json::to_value(&rebound).map_err(|e| e.to_string())?
            != serde_json::to_value(self).map_err(|e| e.to_string())?
        {
            return Err("SOURCE_EDIT_PROPOSAL_BINDING_CHANGED".into());
        }
        let replacement = encoded(&self.proposed_value, self.quote)?;
        let original = fresh.primary_bytes();
        let length = original
            .len()
            .checked_sub(self.value_range.len())
            .and_then(|n| n.checked_add(replacement.len()))
            .ok_or("SOURCE_EDIT_SIZE_OVERFLOW")?;
        if length > v2::MAX_PRIMARY {
            return Err("SOURCE_EDIT_PRIMARY_LIMIT".into());
        }
        let mut output = Vec::with_capacity(length);
        output.extend_from_slice(&original[..self.value_range.start]);
        output.extend_from_slice(replacement.as_bytes());
        output.extend_from_slice(&original[self.value_range.end..]);
        let text = std::str::from_utf8(&output).map_err(|_| "SOURCE_EDIT_OUTPUT_UTF8")?;
        let doc =
            roxmltree::Document::parse(text).map_err(|e| format!("SOURCE_EDIT_OUTPUT_XML: {e}"))?;
        let element_start = self.value_range_element_start(&fresh)?;
        let selected = doc
            .descendants()
            .find(|n| n.is_element() && n.range().start == element_start);
        if selected.and_then(|n| n.attribute("name")) != Some(self.proposed_value.as_str()) {
            return Err("SOURCE_EDIT_OUTPUT_VALUE_CHANGED".into());
        }
        let destination_name = destination
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("SOURCE_EDIT_DESTINATION_NAME")?;
        let result = v2::inspect_primary_bytes(destination_name, output.clone())?;
        let output_hash = format!("{:x}", Sha256::digest(&output));
        if result.primary_sha256 != output_hash
            || result.stage != ProjectionStage::PartialSemanticProjection
        {
            return Err("SOURCE_EDIT_OUTPUT_PROJECTION_REFUSED: no destination created".into());
        }
        crate::new_copy::write_private_new(destination, &output)?;
        let persisted = mbse_model::local_input::read(destination, v2::MAX_PRIMARY, "SOURCE_EDIT_OUTPUT")
            .map_err(|error| format!("SOURCE_EDIT_OUTPUT_CREATED_READ_FAILED: preserve the created output at {} for inspection; {error}", destination.display()))?;
        if persisted != output {
            return Err(
                format!("SOURCE_EDIT_OUTPUT_CREATED_BYTES_CHANGED: preserve the created output at {} for inspection", destination.display()),
            );
        }
        Ok(EditReceipt {
            profile: PROFILE, original_source_set_digest: self.source_set_digest.clone(), output_source_set_digest: result.source_set_digest,
            original_primary_sha256: self.primary_sha256.clone(), output_primary_sha256: output_hash,
            changed_value_range: self.value_range.clone(), original_encoded_bytes: self.value_range.len(), replacement_encoded_bytes: replacement.len(),
            outside_patch_bytes_unchanged: output[..self.value_range.start] == original[..self.value_range.start] && output[self.value_range.start + replacement.len()..] == original[self.value_range.end..],
            xml_reparsed: true, persisted_ids_and_reference_spellings_unchanged: true, engineering_validated: false,
            limitations: vec!["Only an existing unqualified name value changed; byte preservation outside that value is exact".into(), "Persisted IDs and reference bytes are retained. Name-based consumers, Arcadia validity and external-tool acceptance are unverified".into(), "No archive modification, native admission, dependency resolution or execution".into(), "Source preconditions apply to the freshly read frozen bytes; later external source changes are not monitored during persistence".into(), "Create-new placement requires hard-link support in the destination filesystem; existing leaf entries are never followed or replaced".into()],
        })
    }
    fn value_range_element_start(&self, set: &ProjectionSet) -> Result<usize, String> {
        Ok(eligible(set, &self.element)?.location.start)
    }
}

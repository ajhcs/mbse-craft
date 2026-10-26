//! App-owned extension admission; the embedded Stage1 profile is never renamed.
use crate::stage1::Snapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "mbse-craft.extensions/1";
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_BYTES: usize = 256 * 1024;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    pub profile: String,
    pub schema_version: u32,
    /// Unknown optional namespaces are opaque retained JSON. Unknown required ones refuse.
    pub required_namespaces: Vec<String>,
    pub namespaces: BTreeMap<String, Value>,
}
impl Default for Extensions {
    fn default() -> Self {
        Self {
            profile: PROFILE.into(),
            schema_version: SCHEMA_VERSION,
            required_namespaces: vec![],
            namespaces: BTreeMap::new(),
        }
    }
}
impl Extensions {
    pub fn validate(&self) -> Result<(), String> {
        if self.profile != PROFILE || self.schema_version != SCHEMA_VERSION {
            return Err("MBSE_EXTENSIONS_UNSUPPORTED_PROFILE_OR_VERSION: original retained".into());
        }
        if self.namespaces.len() > 16 || self.required_namespaces.len() > 16 {
            return Err("MBSE_EXTENSIONS_NAMESPACE_LIMIT".into());
        }
        for key in self
            .namespaces
            .keys()
            .chain(self.required_namespaces.iter())
        {
            if key.is_empty()
                || key.len() > 128
                || !key
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._/-".contains(&c))
            {
                return Err("MBSE_EXTENSIONS_NAMESPACE_KEY".into());
            }
        }
        let mut seen = BTreeSet::new();
        for required in &self.required_namespaces {
            if !seen.insert(required) || !self.namespaces.contains_key(required) {
                return Err("MBSE_EXTENSIONS_REQUIRED_NAMESPACE_SHAPE".into());
            }
        }
        // P15 and later register actual typed interpreters. A namespace key alone
        // does not establish support for its required engineering meaning.
        if !self.required_namespaces.is_empty() {
            return Err(format!(
                "MBSE_EXTENSIONS_REQUIRED_UNSUPPORTED: {}",
                self.required_namespaces.join(",")
            ));
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_BYTES {
            return Err("MBSE_EXTENSIONS_BYTE_LIMIT".into());
        }
        Ok(())
    }
}
pub fn migrate(source: &Snapshot) -> Result<Snapshot, String> {
    source.validate()?;
    if source.extensions.is_some() {
        return Err("MBSE_EXTENSIONS_ALREADY_VERSIONED: open existing copy explicitly".into());
    }
    let mut copy = source.clone();
    copy.extensions = Some(Extensions::default());
    copy.validate()?;
    Ok(copy)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::DocumentId;
    use serde_json::json;
    #[test]
    fn migration_preserves_native_identity_records_opaque_fields_and_all_journals() {
        let mut old = Snapshot::new(DocumentId("native:retained".into()));
        old.records
            .get_mut(&crate::stage1::RecordId(1))
            .unwrap()
            .fields
            .insert(
                "opaque.future".into(),
                json!({"unknown":[1,null,"untouched"]}),
            );
        let bytes = serde_json::to_vec(&old).unwrap();
        let migrated = migrate(&old).unwrap();
        assert_eq!(migrated.extensions, Some(Extensions::default()));
        let mut core = migrated.clone();
        core.extensions = None;
        assert_eq!(serde_json::to_vec(&core).unwrap(), bytes);
        assert_eq!(serde_json::to_vec(&old).unwrap(), bytes);
        assert!(migrate(&migrated).is_err());
    }
    #[test]
    fn unsupported_versions_required_namespaces_and_budgets_refuse_without_mutation() {
        let mut e = Extensions::default();
        e.schema_version = 2;
        assert!(e.validate().is_err());
        e = Extensions::default();
        e.namespaces
            .insert("future.structure/1".into(), json!({"opaque":"retained"}));
        assert!(e.validate().is_ok());
        let original = e.clone();
        e.required_namespaces.push("future.structure/1".into());
        assert!(e.validate().unwrap_err().contains("REQUIRED_UNSUPPORTED"));
        assert_eq!(e.namespaces, original.namespaces);
        e = Extensions::default();
        e.namespaces.insert("bad key".into(), json!(null));
        assert!(e.validate().is_err());
        e = Extensions::default();
        e.namespaces
            .insert("future/1".into(), json!("x".repeat(MAX_BYTES)));
        assert!(e.validate().is_err());
    }
}

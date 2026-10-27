use mbse_exchange::{source_edit::NameProposal, v2};
use std::fs;
const SOURCE: &str = "<?xml version='1.0'?>\n<!-- unknown bytes must survive -->\n<root xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' xmlns:ctx='http://www.polarsys.org/capella/core/ctx/7.0.0'><ownedMissions xsi:type='ctx:Mission' id='mission-a' name = 'Old &amp; name' extra='opaque'><child id='keep' href='#mission-a'/></ownedMissions></root>\n";
fn key(set: &v2::ProjectionSet) -> v2::ElementKey {
    set.nodes
        .iter()
        .find(|n| n.name == "Old & name")
        .unwrap()
        .key
        .clone()
}
#[test]
fn rename_preserves_other_bytes_original_and_reference_spellings() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    let output = dir.path().join("new.capella");
    fs::write(&source, SOURCE).unwrap();
    let set = v2::open(&source).unwrap();
    let proposal = NameProposal::prepare(&set, &key(&set), "New '&<>\n\t\r名".into()).unwrap();
    let receipt = proposal.create_new_copy(&source, &output).unwrap();
    let bytes = fs::read(&output).unwrap();
    assert_eq!(fs::read(&source).unwrap(), SOURCE.as_bytes());
    assert_eq!(
        &bytes[..proposal.value_range.start],
        &SOURCE.as_bytes()[..proposal.value_range.start]
    );
    assert_eq!(
        &bytes[proposal.value_range.start + receipt.replacement_encoded_bytes..],
        &SOURCE.as_bytes()[proposal.value_range.end..]
    );
    assert!(receipt.outside_patch_bytes_unchanged && receipt.xml_reparsed);
    assert!(!receipt.engineering_validated);
    assert_ne!(
        receipt.output_primary_sha256,
        receipt.original_primary_sha256
    );
    let reopened = v2::open(&output).unwrap();
    assert!(reopened
        .nodes
        .iter()
        .any(|n| n.name == proposal.proposed_value));
    assert_eq!(
        fs::metadata(&output).unwrap().permissions().readonly(),
        false
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(proposal.create_new_copy(&source, &output).is_err());
    assert_eq!(fs::read(&output).unwrap(), bytes);
}
#[test]
fn stale_source_and_forged_range_refuse_before_destination_creation() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    let output = dir.path().join("new.capella");
    fs::write(&source, SOURCE).unwrap();
    let set = v2::open(&source).unwrap();
    let mut proposal = NameProposal::prepare(&set, &key(&set), "New".into()).unwrap();
    proposal.value_range.start += 1;
    assert!(proposal
        .create_new_copy(&source, &output)
        .unwrap_err()
        .contains("BINDING_CHANGED"));
    assert!(!output.exists());
    let proposal = NameProposal::prepare(&set, &key(&set), "New".into()).unwrap();
    fs::write(&source, SOURCE.replace("opaque", "changed")).unwrap();
    assert!(proposal
        .create_new_copy(&source, &output)
        .unwrap_err()
        .contains("STALE_SOURCE"));
    assert!(!output.exists());
}
#[test]
fn missing_name_unknown_class_proxy_and_invalid_characters_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.capella");
    for text in [
        SOURCE.replace(" name = 'Old &amp; name'", ""),
        SOURCE.replace("ctx:Mission", "ctx:Unknown"),
        SOURCE.replace(" extra='opaque'", " href='other.capella#id'"),
    ] {
        fs::write(&path, text).unwrap();
        let set = v2::open(&path).unwrap();
        let key = set
            .nodes
            .iter()
            .find(|n| n.key.identity == v2::ElementIdentity::Original("mission-a".into()))
            .unwrap()
            .key
            .clone();
        assert!(NameProposal::prepare(&set, &key, "New".into()).is_err());
    }
    fs::write(&path, SOURCE).unwrap();
    let set = v2::open(&path).unwrap();
    assert!(NameProposal::prepare(&set, &key(&set), "bad\0text".into()).is_err());
    assert!(NameProposal::prepare(&set, &key(&set), "x".repeat(v2::MAX_VALUE_BYTES + 1)).is_err());
}
#[test]
fn supplied_byte_reader_matches_disk_and_refuses_oversize() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.capella");
    fs::write(&path, SOURCE).unwrap();
    let disk = v2::open(&path).unwrap();
    let memory = v2::inspect_primary_bytes("source.capella", SOURCE.as_bytes().to_vec()).unwrap();
    assert_eq!(disk.source_set_digest, memory.source_set_digest);
    assert_eq!(
        serde_json::to_value(disk).unwrap(),
        serde_json::to_value(memory).unwrap()
    );
    assert!(v2::inspect_primary_bytes("source.capella", vec![0; v2::MAX_PRIMARY + 1]).is_err());
}

#[test]
fn unsupported_xml_declarations_refuse_before_output_creation() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    let output = dir.path().join("new.capella");
    for declaration in [
        "<?xml version='1.1'?>",
        "<?xml version='9.9'?>",
        "<?xml version='1.0' encoding='ISO-8859-1'?>",
        "<?xml version='1.0' encoding='UTF-16'?>",
        "<?xml version='&#49;.0'?>",
    ] {
        fs::write(
            &source,
            SOURCE.replacen("<?xml version='1.0'?>", declaration, 1),
        )
        .unwrap();
        let set = v2::open(&source).unwrap();
        if set.stage == v2::ProjectionStage::PartialSemanticProjection {
            assert!(NameProposal::prepare(&set, &key(&set), "New".into()).is_err());
        } else {
            assert!(set.nodes.is_empty());
        }
        assert!(!output.exists());
    }
}
#[test]
fn bom_no_declaration_quotes_and_original_whitespace_keep_exact_other_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    for (i, original) in [
        SOURCE.to_owned(),
        SOURCE.replace("name = 'Old &amp; name'", "name = \"Old &amp; name\""),
        SOURCE.replacen("<?xml version='1.0'?>\n", "", 1),
        format!(
            "\u{feff}{}",
            SOURCE
                .replace("version='1.0'", "version=\"1.0\" encoding=\"uTf-8\"")
                .replace('\n', "\r\n")
        ),
        SOURCE.replace("Old &amp; name", "Old\r\n&#9; &amp; name"),
    ]
    .into_iter()
    .enumerate()
    {
        fs::write(&source, &original).unwrap();
        let set = v2::open(&source).unwrap();
        let key = set
            .nodes
            .iter()
            .find(|n| n.key.identity == v2::ElementIdentity::Original("mission-a".into()))
            .unwrap()
            .key
            .clone();
        let proposal =
            NameProposal::prepare(&set, &key, "Both ' \" & < > 名\n\r\t".into()).unwrap();
        let output = dir.path().join(format!("new-{i}.capella"));
        let receipt = proposal.create_new_copy(&source, &output).unwrap();
        let bytes = fs::read(&output).unwrap();
        assert_eq!(fs::read(&source).unwrap(), original.as_bytes());
        assert_eq!(
            &bytes[..proposal.value_range.start],
            &original.as_bytes()[..proposal.value_range.start]
        );
        assert_eq!(
            &bytes[proposal.value_range.start + receipt.replacement_encoded_bytes..],
            &original.as_bytes()[proposal.value_range.end..]
        );
        assert!(v2::open(&output)
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.name == proposal.proposed_value));
    }
}
#[cfg(unix)]
#[test]
fn source_edit_and_frozen_raw_copy_refuse_dangling_symlink_without_target_creation() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    let output = dir.path().join("new.capella");
    let target = dir.path().join("never-created.capella");
    fs::write(&source, SOURCE).unwrap();
    let set = v2::open(&source).unwrap();
    let proposal = NameProposal::prepare(&set, &key(&set), "New".into()).unwrap();
    symlink(&target, &output).unwrap();
    assert!(proposal.create_new_copy(&source, &output).is_err());
    assert!(set.copy_original_new(&output).is_err());
    let legacy = mbse_exchange::open_external(&source).unwrap();
    assert!(mbse_exchange::fresh_copy(&legacy, &output).is_err());
    assert!(!target.exists());
    assert_eq!(fs::read(&source).unwrap(), SOURCE.as_bytes());
    assert!(fs::symlink_metadata(&output)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn supported_classes_preserve_original_ids_references_and_unit_lexical_values() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    for class in ["Mission", "Capability", "SystemComponent"] {
        let original = SOURCE
            .replace("ctx:Mission", &format!("ctx:{class}"))
            .replace(
                "extra='opaque'",
                "extra='opaque' unit='m/s' customRef='#keep'",
            );
        fs::write(&source, &original).unwrap();
        let set = v2::open(&source).unwrap();
        let proposal = NameProposal::prepare(&set, &key(&set), format!("Renamed {class}")).unwrap();
        let output = dir.path().join(format!("renamed-{class}.capella"));
        proposal.create_new_copy(&source, &output).unwrap();
        let reopened = v2::open(&output).unwrap();
        let original_ids = |set: &v2::ProjectionSet| -> std::collections::BTreeSet<String> {
            set.nodes
                .iter()
                .filter_map(|n| match &n.key.identity {
                    v2::ElementIdentity::Original(id) => Some(id.clone()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(original_ids(&set), original_ids(&reopened));
        let references = |set: &v2::ProjectionSet| -> Vec<String> {
            set.references
                .iter()
                .filter(|r| r.provenance != v2::ReferenceProvenance::SourceContainment)
                .map(|r| {
                    format!(
                        "{:?}|{}|{}|{:?}|{:?}",
                        r.source.identity,
                        r.feature,
                        r.raw,
                        r.target.as_ref().map(|k| &k.identity),
                        r.resolution
                    )
                })
                .collect()
        };
        assert_eq!(references(&set), references(&reopened));
        let text = fs::read_to_string(&output).unwrap();
        assert!(text.contains("unit='m/s' customRef='#keep'"));
        assert_eq!(fs::read_to_string(&source).unwrap(), original);
    }
}

#[test]
fn anonymous_foreign_namespace_and_archive_elements_refuse_name_edits() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.capella");
    for original in [
        SOURCE.replace(" id='mission-a'", ""),
        SOURCE.replace(
            "http://www.polarsys.org/capella/core/ctx/7.0.0",
            "urn:unrecognized:ctx",
        ),
    ] {
        fs::write(&source, original).unwrap();
        let set = v2::open(&source).unwrap();
        assert!(NameProposal::prepare(&set, &key(&set), "New".into()).is_err());
    }
    let archive = dir.path().join("source.mdzip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    zip.start_file(
        "com.nomagic.magicdraw.uml_model.model",
        zip::write::FileOptions::default(),
    )
    .unwrap();
    zip.write_all(SOURCE.as_bytes()).unwrap();
    zip.finish().unwrap();
    let set = v2::open(&archive).unwrap();
    assert!(NameProposal::prepare(&set, &key(&set), "New".into()).is_err());
}

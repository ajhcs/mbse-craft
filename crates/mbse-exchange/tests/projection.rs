//! Original MIT synthetic fixtures; no proprietary models or copied source assets.
use mbse_exchange::{fresh_copy, open_external, NodeKind};
use std::io::{Cursor, Write};
const UML: &str = r#"<?xml version="1.0" encoding="UTF-8"?><xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:uml="http://www.omg.org/spec/UML/20131001" xmlns:sysml="http://www.omg.org/spec/SysML/20181001/SysML" xmlns:foreign="urn:unknown"><uml:Model xmi:id="m" name="Synthetic"><packagedElement xmi:type="uml:Class" xmi:id="c" name="Vehicle"><ownedAttribute xmi:type="uml:Property" xmi:id="p" name="distance" type="missingUnit" unit="metres"/><ownedAttribute xmi:type="uml:Port" xmi:id="port" name="out"/></packagedElement><packagedElement xmi:type="uml:Actor" xmi:id="a" name="Operator"/><packagedElement xmi:type="uml:InformationFlow" xmi:id="f" source="a" target="port"/><foreign:Class xmi:id="unknown" name="Retain me"/></uml:Model><sysml:Block xmi:id="st" base_Class="c"/></xmi:XMI>"#;
fn area() -> tempfile::TempDir {
    let root = std::env::var_os("TMPDIR").expect("workspace TMPDIR");
    assert!(std::path::Path::new(&root).starts_with("/workspace"));
    tempfile::tempdir_in(root).unwrap()
}
fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        zip.start_file(
            *name,
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
#[test]
fn uml_nodes_id_endpoints_stereotypes_unknown_unit_strings_and_exact_copy() {
    let area = area();
    let path = area.path().join("source.xmi");
    std::fs::write(&path, UML).unwrap();
    let doc = open_external(&path).unwrap();
    assert_eq!(doc.original_bytes, UML.as_bytes());
    assert_eq!(
        doc.nodes.iter().find(|n| n.source_id == "a").unwrap().kind,
        NodeKind::Actor
    );
    assert_eq!(
        doc.nodes.iter().find(|n| n.source_id == "st").unwrap().kind,
        NodeKind::Stereotype
    );
    assert_eq!(
        doc.nodes
            .iter()
            .find(|n| n.source_id == "unknown")
            .unwrap()
            .kind,
        NodeKind::Unknown
    );
    assert_eq!(
        doc.nodes
            .iter()
            .find(|n| n.source_id == "p")
            .unwrap()
            .properties["unit"],
        "metres"
    );
    assert!(doc
        .references
        .iter()
        .any(|r| r.source == "f" && r.target == "port" && r.resolved));
    assert!(doc
        .references
        .iter()
        .any(|r| r.target == "missingUnit" && !r.resolved));
    let copy = area.path().join("copy.xmi");
    fresh_copy(&doc, &copy).unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), UML.as_bytes());
    assert!(fresh_copy(&doc, &copy).is_err());
    assert_eq!(std::fs::read(&copy).unwrap(), UML.as_bytes());
}
#[test]
fn mdzip_projects_actual_named_core_and_retains_unknown_members_without_extraction() {
    let area = area();
    let bytes = archive(&[
        ("com.nomagic.magicdraw.uml_model.model", UML.as_bytes()),
        ("vendor/layout.bin", b"opaque\0layout"),
    ]);
    let path = area.path().join("example.mdzip");
    std::fs::write(&path, &bytes).unwrap();
    let doc = open_external(&path).unwrap();
    assert_eq!(doc.original_bytes, bytes);
    assert_eq!(doc.files.len(), 2);
    assert!(doc.nodes.iter().any(|n| n.source_id == "c"));
    assert!(!area.path().join("vendor").exists());
}
#[test]
fn capella_explicit_components_ports_exchange_and_no_sidecar_fetch() {
    let area = area();
    let path = area.path().join("model.capella");
    std::fs::write(&path,r#"<m:Project xmlns:m="http://www.polarsys.org/capella/core/modeller/7.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:la="http://www.polarsys.org/capella/core/la/7.0.0" id="root"><owned xsi:type="la:LogicalComponent" id="c" name="Coordinator"/><owned xsi:type="la:ComponentPort" id="p"/><owned xsi:type="la:ComponentExchange" id="e" source="p" target="other.capella#remote"/></m:Project>"#).unwrap();
    let doc = open_external(&path).unwrap();
    assert_eq!(
        doc.nodes.iter().find(|n| n.source_id == "c").unwrap().kind,
        NodeKind::Classifier
    );
    assert!(doc
        .references
        .iter()
        .any(|r| r.target == "other.capella#remote" && !r.resolved));
    assert!(doc
        .diagnostics
        .iter()
        .any(|d| d.code == "CAPELLA_AIRD_EXTERNAL"));
}
#[test]
fn xml_refuses_dtd_duplicate_ids_depth_and_encoding() {
    let area = area();
    for (name, bytes) in [
        (
            "dtd.xmi",
            b"<!DOCTYPE a [<!ENTITY e SYSTEM 'file:///etc/passwd'>]><a>&e;</a>".as_slice(),
        ),
        ("duplicate.xmi", b"<a><b id='x'/><b id='x'/></a>".as_slice()),
        (
            "encoding.xmi",
            b"<?xml version='1.0' encoding='ISO-8859-1'?><a/>".as_slice(),
        ),
    ] {
        let path = area.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        assert!(open_external(&path).is_err(), "{name}");
    }
    let path = area.path().join("deep.xmi");
    std::fs::write(
        &path,
        format!("{}{}", "<a>".repeat(110), "</a>".repeat(110)),
    )
    .unwrap();
    assert!(open_external(&path).is_err());
}
#[test]
fn zip_refuses_paths_duplicate_candidates_and_high_ratio() {
    let area = area();
    for (name, members) in [
        ("traversal.mdzip", vec![("../evil", b"x".as_slice())]),
        (
            "ambiguous.mdzip",
            vec![("one.uml", UML.as_bytes()), ("two.uml", UML.as_bytes())],
        ),
    ] {
        let path = area.path().join(name);
        std::fs::write(&path, archive(&members)).unwrap();
        assert!(open_external(&path).is_err());
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        "com.nomagic.magicdraw.uml_model.model",
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
    )
    .unwrap();
    zip.write_all(&vec![b' '; 1_000_000]).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let path = area.path().join("ratio.mdzip");
    std::fs::write(&path, bytes).unwrap();
    assert!(open_external(&path).unwrap_err().contains("RATIO"));
}
#[test]
fn textual_ast_has_declared_spans_and_explicit_unresolved_typing() {
    let area = area();
    let path = area.path().join("model.sysml");
    let source =
        "package Synthetic { part def Vehicle; part vehicle : Vehicle; attribute distance = 12; }";
    std::fs::write(&path, source).unwrap();
    let doc = open_external(&path).unwrap();
    assert!(doc
        .nodes
        .iter()
        .any(|n| n.name == "Vehicle" && n.original_type == "PartDefinition"));
    assert!(doc
        .references
        .iter()
        .any(|r| r.role == "typedBy" && r.target == "Vehicle" && !r.resolved));
    for n in &doc.nodes {
        assert_eq!(
            &source[n.location.start..n.location.end],
            n.properties["sourceText"]
        );
    }
    assert!(doc
        .diagnostics
        .iter()
        .any(|d| d.code == "TEXT_PARTIAL_RESOLUTION"));
}

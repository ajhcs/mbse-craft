//! Original MIT synthetic semantic cases; no private engineering source copied.
use mbse_exchange::v2::*;
use std::io::{Cursor, Write};
const CAPELLA: &str = include_str!("fixtures/v2/capella-semantic-fragment.xml");
const PROFILES: &str = include_str!("fixtures/v2/profile-layering.xmi");
fn area() -> tempfile::TempDir {
    let root = std::env::var_os("TMPDIR").expect("workspace TMPDIR");
    assert!(std::path::Path::new(&root).starts_with("/workspace"));
    tempfile::tempdir_in(root).unwrap()
}
fn load(text: &str) -> ProjectionSet {
    let a = area();
    let path = a.path().join("source.capella");
    std::fs::write(&path, text).unwrap();
    open(&path).unwrap()
}
fn node<'a>(doc: &'a ProjectionSet, id: &str) -> &'a SemanticNode {
    doc.nodes
        .iter()
        .find(|n| n.key.identity == ElementIdentity::Original(id.into()))
        .unwrap()
}
fn refs<'a>(
    doc: &'a ProjectionSet,
    id: &str,
    feature: &'a str,
) -> impl Iterator<Item = &'a SemanticReference> {
    let key = &node(doc, id).key;
    doc.references
        .iter()
        .filter(move |r| &r.source == key && r.feature == feature)
}
fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(vec![]));
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
fn capella_stored_roles_containment_defaults_and_unevaluated_bodies() {
    let doc = load(CAPELLA);
    assert_eq!(doc.stage, ProjectionStage::PartialSemanticProjection);
    assert_eq!(node(&doc, "SYN-M").kind, SemanticKind::CapellaMission);
    assert!(refs(&doc, "SYN-CE", "capability")
        .any(|r| r.resolution == ReferenceResolution::Resolved
            && r.target.as_ref() == Some(&node(&doc, "SYN-C").key)));
    assert!(refs(&doc, "SYN-MI", "container-derived.involver")
        .any(|r| r.provenance == ReferenceProvenance::ContainmentDerived
            && r.target.as_ref() == Some(&node(&doc, "SYN-M").key)));
    assert_eq!(
        node(&doc, "SYN-S").component_flags.as_ref().unwrap().actor,
        FlagValue::MetamodelDefault(false)
    );
    let actor = node(&doc, "SYN-A").component_flags.as_ref().unwrap();
    assert_eq!(actor.actor, FlagValue::Explicit(true));
    assert_eq!(actor.human, FlagValue::Explicit(true));
    assert_eq!(
        node(&doc, "SYN-OUT").source_type.namespace,
        "http://www.polarsys.org/capella/core/fa/7.0.0"
    );
    assert!(refs(&doc, "SYN-X", "source").any(|r| r.resolution == ReferenceResolution::Resolved));
    assert!(refs(&doc, "SYN-ALLOC", "targetElement")
        .any(|r| r.resolution == ReferenceResolution::Resolved));
    let empty = node(&doc, "SYN-O0").opaque_expression.as_ref().unwrap();
    assert_eq!(empty.bodies, vec![""]);
    assert_eq!(empty.languages, vec!["capella:linkedText"]);
    assert!(!empty.evaluated);
    let body = node(&doc, "SYN-O1").opaque_expression.as_ref().unwrap();
    assert!(!body.bodies[0].is_empty());
    assert!(!body.evaluated);
    assert_eq!(node(&doc, "SYN-V").value_kind, Some(ValueKind::Integer));
    assert!(refs(&doc, "SYN-EV", "type").any(|r| r.resolution == ReferenceResolution::Resolved));
    assert_eq!(doc.primary_bytes(), CAPELLA.as_bytes());
    let n = node(&doc, "SYN-M");
    assert!(
        std::str::from_utf8(&doc.primary_bytes()[n.location.start..n.location.end])
            .unwrap()
            .contains("SYN-M")
    );
    assert_eq!(
        serde_json::to_vec(&doc).unwrap().len(),
        doc.counters.serialized_report_bytes
    );
}
#[test]
fn exact_namespaces_wrong_version_and_uml_falsefriends_remain_unsupported() {
    let doc = load(&CAPELLA.replace(
        "http://www.polarsys.org/capella/core/ctx/7.0.0",
        "http://www.omg.org/spec/UML/20131001",
    ));
    assert_eq!(node(&doc, "SYN-M").kind, SemanticKind::UnknownClassifier);
    assert_eq!(node(&doc, "SYN-C").kind, SemanticKind::UnknownClassifier);
    let doc = load(&CAPELLA.replace("/7.0.0", "/99.0.0"));
    assert!(doc
        .nodes
        .iter()
        .all(|n| n.kind == SemanticKind::UnknownClassifier));
    assert!(!doc.source_versions[0].recognized_contract);
    assert!(doc
        .losses
        .iter()
        .any(|l| l.code == "UNSUPPORTED_METAMODEL_CASE"));
}
#[test]
fn profile_layers_keep_exact_base_metaclass_and_coverage_gaps() {
    let doc = load(PROFILES);
    assert_eq!(
        node(&doc, "SYN-BLOCK").kind,
        SemanticKind::SysmlProfileApplication
    );
    assert_eq!(
        node(&doc, "SYN-UAF").kind,
        SemanticKind::UafProfileApplication
    );
    assert_eq!(
        node(&doc, "SYN-UNKNOWN").kind,
        SemanticKind::UnknownProfileApplication
    );
    assert!(refs(&doc, "SYN-BLOCK", "base_Class")
        .any(|r| r.resolution == ReferenceResolution::Resolved));
    // UML Component is a valid Class subtype, but this slice deliberately lacks its subtype proof table.
    let derived = load(&PROFILES.replace("uml:Class", "uml:Component"));
    assert!(refs(&derived, "SYN-BLOCK", "base_Class")
        .any(|r| r.resolution == ReferenceResolution::UnsupportedEndpointType));
    let wrong = load(&PROFILES.replace("uml:Class", "uml:Property"));
    assert!(refs(&wrong, "SYN-BLOCK", "base_Class")
        .any(|r| r.resolution == ReferenceResolution::WrongKind));
    assert_eq!(
        node(&derived, "SYN-BLOCK")
            .attributes
            .iter()
            .find(|a| a.name == "base_Class")
            .unwrap()
            .value,
        "SYN-BASE"
    );
}
#[test]
fn inherited_involved_role_accepts_audited_function_and_rejects_known_wrong_kind() {
    let derived = load(&CAPELLA.replace("involved=\"SYN-A\"", "involved=\"SYN-F\""));
    assert!(
        refs(&derived, "SYN-MI", "involved").any(|r| r.resolution == ReferenceResolution::Resolved)
    );
    let wrong = load(&CAPELLA.replace("involved=\"SYN-A\"", "involved=\"SYN-M\""));
    assert!(refs(&wrong, "SYN-MI", "involved")
        .any(|r| r.resolution == ReferenceResolution::WrongKind && r.target.is_none()));
    let unknown = load(
        &CAPELLA
            .replace("ctx:SystemFunction", "ctx:UnknownDerivedFunction")
            .replace("involved=\"SYN-A\"", "involved=\"SYN-F\""),
    );
    assert!(refs(&unknown, "SYN-MI", "involved")
        .any(|r| r.resolution == ReferenceResolution::UnsupportedEndpointType));
}
#[test]
fn source_digest_excludes_physical_directory_and_includes_opaque_archive_member() {
    let a = area();
    let b = area();
    let p = a.path().join("same.capella");
    let q = b.path().join("same.capella");
    std::fs::write(&p, CAPELLA).unwrap();
    std::fs::write(&q, CAPELLA).unwrap();
    let x = open(&p).unwrap();
    let y = open(&q).unwrap();
    assert_eq!(x.source_set_digest, y.source_set_digest);
    assert_eq!(x.nodes[1].key, y.nodes[1].key);
    let p = a.path().join("same.mdzip");
    let one = archive(&[
        ("com.nomagic.magicdraw.uml_model.model", PROFILES.as_bytes()),
        ("opaque/layout", b"one"),
    ]);
    std::fs::write(&p, &one).unwrap();
    let x = open(&p).unwrap();
    let two = archive(&[
        ("com.nomagic.magicdraw.uml_model.model", PROFILES.as_bytes()),
        ("opaque/layout", b"two"),
    ]);
    std::fs::write(&p, &two).unwrap();
    let y = open(&p).unwrap();
    assert_ne!(x.source_set_digest, y.source_set_digest);
    assert_eq!(x.assets.len(), 3);
    assert_eq!(x.assets[2].bytes, b"one");
    assert_eq!(x.nodes[1].key, y.nodes[1].key); // scoped member identity unaffected by opaque sibling changes
    assert!(x
        .nodes
        .iter()
        .all(|n| n.key.member_path == "members/com.nomagic.magicdraw.uml_model.model"));
}
#[test]
fn duplicate_ids_archive_paths_and_missing_members_refuse_without_losing_primary() {
    let raw = CAPELLA.replace("id=\"SYN-C\"", "id=\"SYN-M\"");
    let doc = load(&raw);
    assert_eq!(doc.stage, ProjectionStage::InterpretationRefused);
    assert!(doc.nodes.is_empty());
    assert_eq!(doc.primary_bytes(), raw.as_bytes());
    let a = area();
    let p = a.path().join("bad.mdzip");
    for members in [
        vec![
            ("com.nomagic.magicdraw.uml_model.model", PROFILES.as_bytes()),
            ("../escape", b"x".as_slice()),
        ],
        vec![("model.xmi", PROFILES.as_bytes())],
        vec![
            ("com.nomagic.magicdraw.uml_model.model", PROFILES.as_bytes()),
            ("opaque", b"x".as_slice()),
            ("opaque/", b"".as_slice()),
        ],
    ] {
        let raw = archive(&members);
        std::fs::write(&p, &raw).unwrap();
        let doc = open(&p).unwrap();
        assert_eq!(doc.stage, ProjectionStage::SourceAdmissionRefused);
        assert_eq!(doc.primary_bytes(), raw);
        assert!(doc.nodes.is_empty());
    }
}
#[test]
fn aggregate_per_node_and_text_limits_refuse_typed_output_but_keep_bytes() {
    for raw in [
        format!(
            "<x {} />",
            (0..129)
                .map(|i| format!("a{i}=\"v\""))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        format!("<x a=\"{}\"/>", "v".repeat(MAX_VALUE_BYTES + 1)),
        format!("<x>{}</x>", "t".repeat(MAX_TEXT_PER_NODE + 1)),
        "<x>".repeat(100) + &"</x>".repeat(100),
    ] {
        let doc = load(&raw);
        assert_eq!(doc.stage, ProjectionStage::OutputBudgetRefused);
        assert!(doc.nodes.is_empty());
        assert_eq!(doc.primary_bytes(), raw.as_bytes());
    }
    let raw = format!(
        "<root>{}</root>",
        (0..600)
            .map(|i| format!("<unknown id=\"{i}\"/>"))
            .collect::<String>()
    );
    let doc = load(&raw);
    assert!(doc.losses.len() <= MAX_LOSSES);
    assert!(doc.counters.omitted_losses > 0);
    assert!(doc.losses.iter().any(|l| l.code == "LOSSES_TRUNCATED"));
    assert_eq!(
        serde_json::to_vec(&doc).unwrap().len(),
        doc.counters.serialized_report_bytes
    );
}
#[test]
fn native_preview_refuses_with_actionable_exact_selection_and_new_copy_is_exact() {
    let doc = load(CAPELLA);
    let selected = node(&doc, "SYN-V").key.clone();
    let preview = doc.admission_preview(&[selected.clone()]);
    assert!(!preview.admitted);
    assert_eq!(preview.selected, vec![selected]);
    assert_eq!(preview.source_set_digest, doc.source_set_digest);
    assert!(preview
        .losses
        .iter()
        .any(|l| l.code == "TYPED_PROPERTY_NOT_MEASURE"));
    let empty = doc.admission_preview(&[]);
    assert!(!empty.admitted);
    assert!(empty
        .losses
        .iter()
        .any(|l| l.code == "NO_EXPLICIT_SELECTION"));
    let a = area();
    let p = a.path().join("new.capella");
    doc.copy_original_new(&p).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), CAPELLA.as_bytes());
    assert!(doc.copy_original_new(&p).is_err());
}
#[test]
fn ordered_slots_unknown_namespace_features_and_global_aggregation_are_bounded() {
    let raw = CAPELLA.replace(
        "<bodies/><languages>capella:linkedText</languages>",
        "<bodies/><bodies>second</bodies><languages>capella:linkedText</languages>",
    );
    let doc = load(&raw);
    let e = node(&doc, "SYN-O0").opaque_expression.as_ref().unwrap();
    assert_eq!(e.bodies, vec!["", "second"]);
    assert_eq!(e.languages, vec!["capella:linkedText"]);
    assert!(!e.evaluated);
    assert!(doc
        .diagnostics
        .iter()
        .any(|d| d.code == "OPAQUE_BODY_LANGUAGE_CARDINALITY"));
    let raw=CAPELLA.replace("<bodies/>","<other:bodies xmlns:other=\"urn:synthetic:unqualified-feature-falsefriend\">opaque</other:bodies>");
    let doc = load(&raw);
    assert!(node(&doc, "SYN-O0")
        .opaque_expression
        .as_ref()
        .unwrap()
        .bodies
        .is_empty());
    assert_eq!(doc.primary_bytes(), raw.as_bytes());
    let value = "v".repeat(32 * 1024);
    let raw = format!(
        "<root>{}</root>",
        (0..513)
            .map(|i| format!("<n id=\"{i}\" value=\"{value}\"/>"))
            .collect::<String>()
    );
    let doc = load(&raw);
    assert_eq!(doc.stage, ProjectionStage::OutputBudgetRefused);
    assert!(doc.nodes.is_empty());
    assert_eq!(doc.primary_bytes(), raw.as_bytes());
    let text = "t".repeat(MAX_TEXT_PER_NODE - 1);
    let raw = format!(
        "<root>{}</root>",
        (0..17)
            .map(|i| format!("<n id=\"{i}\">{text}</n>"))
            .collect::<String>()
    );
    let doc = load(&raw);
    assert_eq!(doc.stage, ProjectionStage::OutputBudgetRefused);
    assert!(doc.nodes.is_empty());
    assert_eq!(doc.primary_bytes(), raw.as_bytes());
}
#[test]
fn exact_json_expansion_limit_and_diagnostic_truncation_preserve_raw_source() {
    // Backslashes are legal XML and expand during JSON serialization. Aggregate
    // XML text remains below 16 MiB while the prospective report exceeds 32 MiB.
    let text = "\\".repeat(MAX_TEXT_PER_NODE - 1);
    let raw = format!(
        "<root>{}</root>",
        (0..16)
            .map(|i| format!("<n id=\"{i}\">{text}</n>"))
            .collect::<String>()
    );
    let doc = load(&raw);
    assert_eq!(doc.stage, ProjectionStage::OutputBudgetRefused);
    assert!(doc.nodes.is_empty());
    assert_eq!(doc.primary_bytes(), raw.as_bytes());
    assert!(serde_json::to_vec(&doc).unwrap().len() <= MAX_REPORT_BYTES);
    let raw=format!("<root xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:dv=\"http://www.polarsys.org/capella/core/information/datavalue/7.0.0\">{}</root>",(0..600).map(|i|format!("<e id=\"{i}\" xsi:type=\"dv:OpaqueExpression\"><bodies>body</bodies></e>")).collect::<String>());
    let doc = load(&raw);
    assert!(doc.diagnostics.len() <= MAX_DIAGNOSTICS);
    assert!(doc.counters.omitted_diagnostics > 0);
    assert!(doc
        .diagnostics
        .iter()
        .any(|d| d.code == "DIAGNOSTICS_TRUNCATED"));
    assert_eq!(
        doc.counters.serialized_report_bytes,
        serde_json::to_vec(&doc).unwrap().len()
    );
}
#[test]
fn content_dispatch_ignores_misleading_suffix_and_keeps_v2_text_refusal() {
    let a = area();
    let path = a.path().join("renamed.sysml");
    let raw =
        format!("\u{feff} \n{PROFILES}").replace("<?xml version=\"1.0\" encoding=\"UTF-8\"?>", "");
    std::fs::write(&path, &raw).unwrap();
    let doc = open(&path).unwrap();
    assert_eq!(doc.stage, ProjectionStage::PartialSemanticProjection);
    assert_eq!(
        node(&doc, "SYN-BLOCK").kind,
        SemanticKind::SysmlProfileApplication
    );
    assert_eq!(doc.primary_bytes(), raw.as_bytes());
    let path = a.path().join("renamed.mdzip");
    std::fs::write(&path, CAPELLA).unwrap();
    assert_eq!(
        open(&path).unwrap().stage,
        ProjectionStage::PartialSemanticProjection
    );
    let path = a.path().join("archive.xml");
    let raw = archive(&[("com.nomagic.magicdraw.uml_model.model", PROFILES.as_bytes())]);
    std::fs::write(&path, &raw).unwrap();
    assert_eq!(open(&path).unwrap().assets.len(), 2);
    for name in ["misleading.xml", "misleading.mdzip", "native.sysml"] {
        let path = a.path().join(name);
        let raw = b"package Native { part def Example; }";
        std::fs::write(&path, raw).unwrap();
        let doc = open(&path).unwrap();
        assert_eq!(doc.stage, ProjectionStage::InterpretationRefused);
        assert!(doc.nodes.is_empty());
        assert_eq!(doc.primary_bytes(), raw);
        assert!(doc
            .losses
            .iter()
            .any(|l| l.code == "FORMAT_ADAPTER_UNAVAILABLE"));
    }
}
#[test]
fn child_namespace_declarations_obey_per_value_and_scope_limits() {
    let long_uri = format!("urn:{}", "n".repeat(MAX_VALUE_BYTES));
    for raw in [format!("<root><child xmlns:long=\"{long_uri}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"long:Class\" id=\"c\"/></root>"),format!("<root><child {} /></root>",(0..129).map(|i|format!("xmlns:n{i}=\"urn:synthetic:{i}\"")).collect::<Vec<_>>().join(" "))] {
        let doc=load(&raw);assert_eq!(doc.stage,ProjectionStage::OutputBudgetRefused);assert!(doc.nodes.is_empty());assert_eq!(doc.primary_bytes(),raw.as_bytes());assert!(doc.diagnostics.iter().any(|d|d.detail.contains("NAMESPACE")));
    }
}
#[test]
fn identity_carrier_policy_is_order_independent_and_does_not_impose_ncname_on_estring() {
    for replacement in [
        "id=\"SYN-M\" xmi:id=\"other\"",
        "xmi:id=\"other\" id=\"SYN-M\"",
        "id=\"SYN-M\" xmi:id=\"SYN-M\"",
    ] {
        let raw = CAPELLA.replace("id=\"SYN-M\"", replacement);
        let doc = load(&raw);
        assert_eq!(doc.stage, ProjectionStage::InterpretationRefused);
        assert!(doc.nodes.is_empty());
        assert_eq!(doc.primary_bytes(), raw.as_bytes());
        assert!(doc
            .diagnostics
            .iter()
            .any(|d| d.detail.contains("INSTANCE_IDENTITY_CARRIERS_CONFLICT")));
    }
    let raw = CAPELLA.replace("id=\"SYN-M\"", "id=\"1 EString value:with spaces\"");
    let doc = load(&raw);
    assert_eq!(doc.stage, ProjectionStage::PartialSemanticProjection);
    assert_eq!(
        node(&doc, "1 EString value:with spaces").kind,
        SemanticKind::CapellaMission
    );
    let raw = CAPELLA.replace("id=\"SYN-M\"", "xmi:id=\"SYN-M\"");
    let doc = load(&raw);
    assert!(doc
        .diagnostics
        .iter()
        .any(|d| d.detail.contains("IDENTITY_CARRIER_COVERAGE_UNSUPPORTED")));
    assert!(doc.nodes.is_empty());
    for replacement in [
        "xmi:id=\"SYN-BASE\" id=\"other\"",
        "id=\"other\" xmi:id=\"SYN-BASE\"",
    ] {
        let doc = load(&PROFILES.replace("xmi:id=\"SYN-BASE\"", replacement));
        assert!(doc
            .diagnostics
            .iter()
            .any(|d| d.detail.contains("INSTANCE_IDENTITY_CARRIERS_CONFLICT")));
        assert!(doc.nodes.is_empty());
    }
}
#[test]
fn reference_uri_policy_is_literal_local_subset_with_no_percent_decode_or_invalidity_claim() {
    for spelling in [
        "#SYN%2DC",
        "#SYN%252DC",
        "#SYN-C#second",
        "#ns:SYN-C",
        "#path/SYN-C",
    ] {
        let raw = CAPELLA.replace(
            "capability=\"SYN-C\"",
            &format!("capability=\"{spelling}\""),
        );
        let doc = load(&raw);
        let r = refs(&doc, "SYN-CE", "capability").next().unwrap();
        assert_eq!(r.resolution, ReferenceResolution::UnsupportedSyntax);
        assert_eq!(r.raw, spelling);
        assert!(r.target.is_none());
        assert!(doc
            .losses
            .iter()
            .any(|l| l.code == "REFERENCE_SYNTAX_COVERAGE_UNSUPPORTED"));
    }
    let raw = CAPELLA.replace("capability=\"SYN-C\"", "capability=\"other.capella#SYN-C\"");
    let doc = load(&raw);
    assert!(refs(&doc, "SYN-CE", "capability")
        .any(|r| r.resolution == ReferenceResolution::UnsuppliedResource));
    let raw = CAPELLA.replace("capability=\"SYN-C\"", "capability=\"#SYN-C\"");
    let doc = load(&raw);
    assert!(
        refs(&doc, "SYN-CE", "capability").any(|r| r.resolution == ReferenceResolution::Resolved)
    );
    let raw = CAPELLA.replace("capability=\"SYN-C\"", "capability=\"missing\"");
    let doc = load(&raw);
    assert!(
        refs(&doc, "SYN-CE", "capability").any(|r| r.resolution == ReferenceResolution::Unresolved)
    );
}

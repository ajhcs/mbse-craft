//! Synthetic local register hierarchy over the existing assurance authority.
use mbse_assurance::*;
use serde_json::{Value, json};
fn area() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap()
}
fn tree() -> (Board, RequirementId, RequirementId, RequirementId) {
    let mut b = Board::new();
    let a = b
        .add_requirement("Root subsystem shall retain source authority".into())
        .unwrap();
    let c = b
        .add_requirement_under("Child shall retain evidence".into(), Some(a))
        .unwrap();
    let g = b
        .add_requirement_under("Grandchild shall retain invalid drafts".into(), Some(c))
        .unwrap();
    (b, a, c, g)
}
fn envelope(b: &Board) -> Value {
    json!({"profile":PROFILE,"register":b.register})
}
#[test]
fn atomic_child_identity_parent_and_preorder_have_independent_initial_states() {
    let (mut b, a, c, g) = tree();
    assert_eq!((a.0, c.0, g.0), (1, 2, 3));
    assert_eq!(
        b.hierarchy()
            .unwrap()
            .iter()
            .map(|r| (r.id.0, r.depth, r.child_count))
            .collect::<Vec<_>>(),
        vec![(1, 0, 1), (2, 1, 1), (3, 2, 0)]
    );
    b.set_maturity(a, Maturity::Approved).unwrap();
    b.set_implementation(a, ImplementationStatus::Implemented)
        .unwrap();
    b.set_verification(a, VerificationStatus::InProgress)
        .unwrap();
    let fresh = b
        .add_requirement_under("Fresh child starts independently".into(), Some(a))
        .unwrap();
    let r = &b.register.requirements[&fresh];
    assert_eq!(r.maturity, Maturity::Draft);
    assert_eq!(r.implementation, ImplementationStatus::UnderReview);
    assert_eq!(r.verification, VerificationStatus::NotStarted);
}
#[test]
fn self_descendant_missing_noncanonical_and_incomplete_parent_proposals_remain_raw() {
    let (mut b, a, c, g) = tree();
    let reqs = b.register.requirements.clone();
    for text in [
        a.0.to_string(),
        c.0.to_string(),
        g.0.to_string(),
        "999".into(),
        "01".into(),
        "2?".into(),
        " ".into(),
    ] {
        assert!(!b.edit_parent(a, text.clone()).unwrap());
        assert_eq!(b.register.requirements, reqs);
        let d = &b.register.drafts["1:parent"];
        assert_eq!(d.text, text);
        assert!(!d.valid);
    }
    assert_eq!(b.register.requirements[&a].parent, None);
    assert_eq!(b.register.requirements[&c].parent, Some(a));
    assert_eq!(b.register.requirements[&g].parent, Some(c));
}
#[test]
fn moving_subtree_and_promoting_root_are_atomic_register_history_edits() {
    let (mut b, a, c, g) = tree();
    let other = b.add_requirement("Other root".into()).unwrap();
    let before = b.register.clone();
    assert!(b.edit_parent(c, other.0.to_string()).unwrap());
    assert_eq!(b.register.requirements[&c].parent, Some(other));
    assert_eq!(b.register.requirements[&g].parent, Some(c));
    assert_eq!(
        b.register.requirements[&c].revision,
        before.requirements[&c].revision + 1
    );
    let moved = b.register.clone();
    assert!(b.undo());
    let mut expected = before.clone();
    expected.next_id = moved.next_id;
    assert_eq!(b.register, expected);
    assert!(b.redo());
    assert_eq!(b.register, moved);
    assert!(b.edit_parent(c, String::new()).unwrap());
    assert_eq!(b.register.requirements[&c].parent, None);
    assert_eq!(b.register.requirements[&a].parent, None);
}
#[test]
fn duplicate_or_invalid_id_state_never_overwrites_existing_records() {
    let (mut b, a, _, _) = tree();
    b.register.next_id = a.0;
    let before = b.register.clone();
    assert!(b.add_requirement("Must not replace root".into()).is_err());
    assert_eq!(b.register, before);
    let (mut b, a, c, _) = tree();
    b.register.requirements.get_mut(&a).unwrap().id = c;
    let before = b.register.clone();
    assert!(b.set_maturity(a, Maturity::Approved).is_err());
    assert_eq!(b.register, before);
    let dir = area();
    let path = dir.path().join("invalid.json");
    assert!(b.save_new(&path).is_err());
    assert!(!path.exists());
}
#[test]
fn reader_refuses_duplicate_json_id_aliases_missing_parent_and_cycles_without_relabeling() {
    let (b, a, c, g) = tree();
    let dir = area();
    let mut cases = Vec::new();
    let mut alias = envelope(&b);
    let first = alias["register"]["requirements"]["1"].clone();
    alias["register"]["requirements"]["01"] = first;
    cases.push(serde_json::to_vec(&alias).unwrap());
    let mut mismatch = envelope(&b);
    mismatch["register"]["requirements"]["1"]["id"] = json!(2);
    cases.push(serde_json::to_vec(&mismatch).unwrap());
    let mut missing = envelope(&b);
    missing["register"]["requirements"][c.0.to_string()]["parent"] = json!(999);
    cases.push(serde_json::to_vec(&missing).unwrap());
    let mut cycle = envelope(&b);
    cycle["register"]["requirements"][a.0.to_string()]["parent"] = json!(g.0);
    cases.push(serde_json::to_vec(&cycle).unwrap());
    let text = serde_json::to_string(&envelope(&b)).unwrap();
    let duplicate = text.replacen(
        "\"requirements\":{",
        &format!(
            "\"requirements\":{{\"1\":{},",
            serde_json::to_string(&b.register.requirements[&a]).unwrap()
        ),
        1,
    );
    cases.push(duplicate.into_bytes());
    for (i, bytes) in cases.iter().enumerate() {
        let path = dir.path().join(format!("bad-{i}.json"));
        std::fs::write(&path, bytes).unwrap();
        assert!(Board::open(&path).is_err(), "{i}");
        assert_eq!(std::fs::read(path).unwrap(), *bytes);
    }
}
#[test]
fn invalid_parent_statement_and_new_statement_drafts_survive_atomic_save_cold_open() {
    let (mut b, a, c, _) = tree();
    let accepted = b.register.requirements[&c].statement.clone();
    assert!(!b.edit_field(c, "statement", "  ".into()).unwrap());
    assert!(!b.edit_parent(a, c.0.to_string()).unwrap());
    b.set_new_statement_draft("  ".into()).unwrap();
    let before = b.register.clone();
    assert!(b.add_requirement_under("  ".into(), Some(a)).is_err());
    assert_eq!(b.register, before);
    b.register
        .unknown
        .insert("future.domain".into(), json!({"opaque":[null,1]}));
    b.register
        .requirements
        .get_mut(&a)
        .unwrap()
        .unknown
        .insert("source.future".into(), json!({"kept":true}));
    let dir = area();
    let path = dir.path().join("hierarchy.json");
    let hash = b.save_new(&path).unwrap();
    let (cold, read_hash) = Board::open(&path).unwrap();
    assert_eq!(hash, read_hash);
    assert_eq!(cold.register, b.register);
    assert_eq!(cold.register.requirements[&c].statement, accepted);
    assert!(!cold.register.drafts["2:statement"].valid);
    assert!(!cold.register.drafts["1:parent"].valid);
    let mut cold = cold;
    assert!(!cold.undo());
    let wire = envelope(&cold);
    assert!(
        wire["register"]["requirements"]["1"]
            .get("parent")
            .is_none()
    );
    assert_eq!(wire["register"]["requirements"]["2"]["parent"], json!(1));
}
#[test]
fn deep_hierarchy_and_cycle_check_are_iterative_and_preserve_budget_limits() {
    let mut b = Board::new();
    let seed = b
        .add_requirement("Bounded synthetic requirement".into())
        .unwrap();
    let template = b.register.requirements[&seed].clone();
    for id in 2..=10000 {
        let mut r = template.clone();
        r.id = RequirementId(id);
        r.parent = Some(RequirementId(id - 1));
        b.register.requirements.insert(r.id, r);
    }
    b.register.next_id = 10001;
    let rows = b.hierarchy().unwrap();
    assert_eq!(rows.len(), 10000);
    assert_eq!(rows.last().unwrap().depth, 9999);
    b.register.requirements.get_mut(&seed).unwrap().parent = Some(RequirementId(10000));
    assert!(b.hierarchy().unwrap_err().contains("CYCLE"));
    b.register.requirements.get_mut(&seed).unwrap().parent = None;
    let mut extra = template;
    extra.id = RequirementId(10001);
    b.register.requirements.insert(extra.id, extra);
    b.register.next_id = 10002;
    assert!(b.hierarchy().unwrap_err().contains("LIMIT"));
}
#[test]
fn reparenting_stales_existing_requirement_evidence_without_status_inheritance() {
    use mbse_model::{
        stage1::{Editor, Snapshot},
        stage1_calc::CalculationId,
    };
    let v: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p23/fixtures/profile-tags.kernspan"
    ))
    .unwrap();
    let s: Snapshot = serde_json::from_value(v["snapshot"].clone()).unwrap();
    let e = Editor::new(s, "p23:evidence".into()).unwrap();
    let (mut b, a, c, _) = tree();
    let other = b.add_requirement("Independent branch".into()).unwrap();
    b.set_maturity(a, Maturity::Approved).unwrap();
    b.set_implementation(a, ImplementationStatus::Implemented)
        .unwrap();
    let config = json!({"purpose":"P23 synthetic parent change"});
    let evidence = b
        .attach_calculation(c, &e, &config, CalculationId(1003))
        .unwrap();
    let context = EvidenceContext::from_editor(&e, &config).unwrap();
    assert!(
        !b.evidence_staleness(evidence, &context)
            .unwrap()
            .iter()
            .any(|s| s == "requirement changed")
    );
    let before = b.register.requirements[&c].clone();
    assert!(b.edit_parent(c, other.0.to_string()).unwrap());
    let after = &b.register.requirements[&c];
    assert_eq!(
        (after.maturity, after.implementation, after.verification),
        (before.maturity, before.implementation, before.verification)
    );
    assert!(
        b.evidence_staleness(evidence, &context)
            .unwrap()
            .iter()
            .any(|s| s == "requirement changed")
    );
    assert_eq!(
        b.register.evidence[&evidence].requirement_revision,
        before.revision
    );
}
#[test]
fn failed_child_admission_and_parent_raw_limits_do_not_allocate_or_clear_work() {
    let (mut b, a, _, _) = tree();
    b.set_new_statement_draft("Retain this child proposal".into())
        .unwrap();
    let before = b.register.clone();
    assert!(
        b.add_requirement_under(
            "Retain this child proposal".into(),
            Some(RequirementId(999))
        )
        .is_err()
    );
    assert_eq!(b.register, before);
    assert!(b.edit_parent(a, "1".repeat(4097)).is_err());
    assert_eq!(b.register, before);
    b.register.next_id = u64::MAX;
    let before = b.register.clone();
    assert!(b.add_requirement("Allocator exhausted".into()).is_err());
    assert_eq!(b.register, before);
}

#[test]
fn serialized_valid_draft_flags_cannot_promote_cycles_missing_parent_or_unaccepted_statement() {
    let (b, _, _, _) = tree();
    let dir = area();
    for (i, (key, text)) in [
        ("1:parent", "2"),
        ("2:parent", "999"),
        ("1:statement", "  "),
        ("1:statement", "Unaccepted different statement"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut data = envelope(&b);
        data["register"]["drafts"][key] = json!({"text":text,"valid":true});
        let bytes = serde_json::to_vec(&data).unwrap();
        let path = dir.path().join(format!("forged-validity-{i}.json"));
        std::fs::write(&path, &bytes).unwrap();
        assert!(
            Board::open(&path)
                .unwrap_err()
                .contains("DRAFT_VALIDITY_MISMATCH")
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let mut exact = envelope(&b);
    exact["register"]["drafts"]["2:parent"] = json!({"text":"1","valid":true});
    let path = dir.path().join("matching-validity.json");
    std::fs::write(&path, serde_json::to_vec(&exact).unwrap()).unwrap();
    assert!(Board::open(&path).is_ok());
}

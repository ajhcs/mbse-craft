use mbse_model::{
    native_views::{
        self as v, Geometry, Layout, NativeAction, NativeOutcome, OccurrenceId, SemanticKey,
        SemanticRole, Views,
    },
    native_views_copy::{EdgeOutcome, OccurrenceOutcome},
    native_views_inspect::Inspection,
    native_views_io::{self as io, Io, Kind},
    native_views_plan::{Choice, Plan, ReviewedCopy},
    stage1::{Command, Editor, RawKey, RecordId, RecordKind, Snapshot},
    stage1_io::{self, Session},
};
use serde_json::json;

struct Fixture {
    area: tempfile::TempDir,
    session: Session,
    views: Views,
    source: Inspection,
}
impl Fixture {
    fn new(duplicate: bool) -> Self {
        let area = tempfile::Builder::new()
            .prefix("p08-")
            .tempdir_in(std::env::var_os("TMPDIR").expect("workspace TMPDIR"))
            .unwrap();
        let mut session = Session::new(&area.path().join("state")).unwrap();
        let snapshot = Snapshot::from_fixture(
            session.editor.snapshot.document_id.clone(),
            &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
        )
        .unwrap();
        session.editor = Editor::new(snapshot, session.editor.freshness().instance_id).unwrap();
        session.editor.select(RecordId(301)).unwrap();
        let mut source_views = Views::new(&session.editor).unwrap();
        let primary = source_views.document().primary;
        let row = |record| {
            source_views
                .document()
                .occurrences
                .values()
                .find(|o| o.record == RecordId(record) && o.view == primary)
                .unwrap()
                .id
        };
        let a = row(301);
        let b = row(300);
        let key = SemanticKey::checked(
            &session.editor.snapshot,
            RecordId(301),
            SemanticRole::MeasureForObjective,
            RecordId(300),
        )
        .unwrap();
        source_views
            .apply(
                &session.editor,
                &source_views.stamp(&session.editor),
                Layout::Route {
                    view: primary,
                    key,
                    source: a,
                    target: b,
                },
            )
            .unwrap();
        let mut doc = source_views.recovery(&session.editor).unwrap();
        doc.document_id.0 = "independent-old-source".into();
        let item = doc.occurrences.get_mut(&b).unwrap();
        item.geometry = Geometry::at(711., 322.);
        item.visible = false;
        item.pinned = true;
        if duplicate {
            let mut value = serde_json::to_value(&doc).unwrap();
            let mut row = value["occurrences"][b.get().to_string()].clone();
            row["id"] = "900".into();
            value["occurrences"]["900"] = row;
            value["next_id"] = "901".into();
            doc = serde_json::from_value(value).unwrap();
        }
        let path = area.path().join("old-source.mbse-views.json");
        std::fs::write(&path, doc.encoded().unwrap()).unwrap();
        let source = Inspection::open(&path).unwrap();
        let views = Views::new(&session.editor).unwrap();
        Self {
            area,
            session,
            views,
            source,
        }
    }
    fn row(&self, record: u64) -> OccurrenceId {
        self.source
            .declared()
            .occurrences
            .values()
            .find(|o| o.view == self.source.declared().primary && o.record == RecordId(record))
            .unwrap()
            .id
    }
    fn selected(&self) -> (Plan, ReviewedCopy) {
        let mut p = Plan::new(
            &self.source,
            self.source.declared().primary,
            &self.session.editor,
            &self.views,
        )
        .unwrap();
        for id in [300, 301] {
            p.choose(
                &self.source,
                &self.session.editor,
                &self.views,
                self.row(id),
                Choice::Target(RecordId(id)),
            )
            .unwrap();
        }
        p.choose(
            &self.source,
            &self.session.editor,
            &self.views,
            self.row(100),
            Choice::Skip("s".repeat(1024)),
        )
        .unwrap();
        let r = p
            .review(
                &self.source,
                &self.session.editor,
                &self.views,
                "Reviewed copy",
            )
            .unwrap();
        (p, r)
    }
    fn state(&self) -> (String, v::Stamp, (usize, usize), u64, Option<RecordId>) {
        (
            self.views.document().digest().unwrap(),
            self.views.stamp(&self.session.editor),
            self.views.history_usage(),
            self.views.document().next_id(),
            self.session.editor.selected,
        )
    }
}

#[test]
fn selected_occurrences_only_copy_with_provenance_and_one_undo_redo() {
    let mut f = Fixture::new(false);
    let old = f.views.document().clone();
    let native = f.session.editor.snapshot.digest();
    let bytes = std::fs::read(f.source.path()).unwrap();
    let (p, r) = f.selected();
    let id = p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap();
    assert_eq!(f.views.document().views.len(), old.views.len() + 1);
    assert_eq!(f.views.history_usage().0, 1);
    assert_eq!(f.views.document().profile, v::COPY_PROFILE);
    for (key, view) in &old.views {
        assert_eq!(f.views.document().views[key], *view);
    }
    for (key, o) in &old.occurrences {
        assert_eq!(f.views.document().occurrences[key], *o);
    }
    let copied: Vec<_> = f
        .views
        .document()
        .occurrences
        .values()
        .filter(|o| o.view == id)
        .collect();
    assert_eq!(copied.len(), 2);
    assert!(copied.iter().all(|o| o.id.get() > old.next_id()));
    let objective = copied.iter().find(|o| o.record == RecordId(300)).unwrap();
    assert!(!objective.visible && objective.pinned && objective.presence.active());
    assert_eq!(objective.geometry, Geometry::at(711., 322.));
    let origin = f.views.document().views[&id].copy_origin.as_ref().unwrap();
    origin.verify_source(&f.source).unwrap();
    assert!(
        matches!(&origin.occurrences[&f.row(100)].outcome,OccurrenceOutcome::Skipped{reason} if reason.len()==1024)
    );
    assert_eq!(
        f.views
            .document()
            .edges
            .values()
            .filter(|e| e.view == id)
            .count(),
        1
    );
    let copied_doc = f.views.document().clone();
    let high = f.views.document().next_id();
    f.views
        .undo(&f.session.editor, &f.views.stamp(&f.session.editor), false)
        .unwrap();
    assert_eq!(f.views.document().views, old.views);
    assert_eq!(f.views.document().occurrences, old.occurrences);
    assert_eq!(f.views.document().profile, v::COPY_PROFILE);
    assert_eq!(f.views.document().next_id(), high);
    f.views
        .undo(&f.session.editor, &f.views.stamp(&f.session.editor), true)
        .unwrap();
    assert_eq!(f.views.document().views, copied_doc.views);
    assert_eq!(f.views.document().occurrences, copied_doc.occurrences);
    assert_eq!(f.views.document().edges, copied_doc.edges);
    assert_eq!(f.views.document().next_id(), high);
    assert_eq!(f.session.editor.snapshot.digest(), native);
    assert_eq!(f.session.editor.selected, Some(RecordId(301)));
    assert_eq!(std::fs::read(f.source.path()).unwrap(), bytes);
}

#[test]
fn default_unselected_cancel_and_empty_apply_have_no_target_effects() {
    let mut f = Fixture::new(false);
    let before = f.state();
    let p = Plan::new(
        &f.source,
        f.source.declared().primary,
        &f.session.editor,
        &f.views,
    )
    .unwrap();
    assert!(p
        .choices()
        .values()
        .all(|c| matches!(c, Choice::Unselected)));
    let r = p
        .review(&f.source, &f.session.editor, &f.views, "Empty")
        .unwrap();
    assert_eq!(f.state(), before);
    assert!(p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap_err()
        .contains("NO_MAPPINGS"));
    assert_eq!(f.state(), before);
    drop(r);
    drop(p);
    assert_eq!(f.state(), before);
}

#[test]
fn changed_choices_foreign_plan_and_reopened_source_refuse_old_review() {
    let mut f = Fixture::new(false);
    let (mut p, r) = f.selected();
    p.choose(
        &f.source,
        &f.session.editor,
        &f.views,
        f.row(300),
        Choice::Unselected,
    )
    .unwrap();
    let before = f.state();
    assert!(p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap_err()
        .contains("REVIEW_STALE"));
    assert_eq!(f.state(), before);
    let (q, new) = f.selected();
    assert!(p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &new)
        .is_err());
    let reopened = Inspection::open(f.source.path()).unwrap();
    assert!(q
        .apply_copy(&reopened, &f.session.editor, &mut f.views, &new)
        .is_err());
    assert_eq!(f.state(), before);
}

#[test]
fn changed_source_and_native_raw_or_layout_refuse_before_history_or_ids() {
    let mut f = Fixture::new(false);
    let (p, r) = f.selected();
    let before = f.state();
    let mut bytes = std::fs::read(f.source.path()).unwrap();
    bytes.push(b'\n');
    std::fs::write(f.source.path(), &bytes).unwrap();
    assert!(p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap_err()
        .contains("SOURCE_CHANGED"));
    assert_eq!(f.state(), before);
    for mode in 0..3 {
        let mut f = Fixture::new(false);
        let (p, r) = f.selected();
        match mode {
            0 => {
                let stamp = f.session.editor.freshness();
                f.session
                    .editor
                    .command(
                        &stamp,
                        Command::EditText {
                            key: RawKey {
                                record: RecordId(301),
                                field: "criterion".into(),
                                item: None,
                            },
                            text: "8e".into(),
                        },
                    )
                    .unwrap();
            }
            1 => {
                let stamp = f.session.editor.freshness();
                f.session
                    .editor
                    .command(
                        &stamp,
                        Command::EditText {
                            key: RawKey {
                                record: RecordId(100),
                                field: "name".into(),
                                item: None,
                            },
                            text: "Changed native".into(),
                        },
                    )
                    .unwrap();
            }
            _ => f
                .views
                .apply(
                    &f.session.editor,
                    &f.views.stamp(&f.session.editor),
                    Layout::Camera(
                        f.views.document().primary,
                        v::Camera::checked(40., 30., 1.).unwrap(),
                    ),
                )
                .unwrap(),
        };
        let before = f.state();
        assert!(p
            .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
            .is_err());
        assert_eq!(f.state(), before);
    }
}

#[test]
fn duplicate_native_targets_keep_exact_source_edge_occurrence_pair() {
    let mut f = Fixture::new(true);
    let (mut p, _) = f.selected();
    let duplicate: OccurrenceId = serde_json::from_value(json!("900")).unwrap();
    p.choose(
        &f.source,
        &f.session.editor,
        &f.views,
        duplicate,
        Choice::Target(RecordId(300)),
    )
    .unwrap();
    let r = p
        .review(&f.source, &f.session.editor, &f.views, "Duplicate targets")
        .unwrap();
    let id = p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap();
    let origin = f.views.document().views[&id].copy_origin.as_ref().unwrap();
    let original = match origin.occurrences[&f.row(300)].outcome {
        OccurrenceOutcome::Mapped {
            target_occurrence, ..
        } => target_occurrence,
        _ => panic!(),
    };
    let other = match origin.occurrences[&duplicate].outcome {
        OccurrenceOutcome::Mapped {
            target_occurrence, ..
        } => target_occurrence,
        _ => panic!(),
    };
    assert_ne!(original, other);
    let edge = origin
        .edges
        .values()
        .find_map(|e| match e.outcome {
            EdgeOutcome::Mapped { target_edge } => Some(&f.views.document().edges[&target_edge]),
            _ => None,
        })
        .unwrap();
    assert_eq!(edge.target, original);
    assert_ne!(edge.target, other);
}

#[test]
fn capacity_allocator_and_title_failures_are_atomic() {
    let mut f = Fixture::new(false);
    for i in 2..v::MAX_VIEWS {
        f.views
            .apply(
                &f.session.editor,
                &f.views.stamp(&f.session.editor),
                Layout::AddView(format!("View{i}")),
            )
            .unwrap();
    }
    let (p, r) = f.selected();
    let before = f.state();
    assert!(p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap_err()
        .contains("CAPACITY"));
    assert_eq!(f.state(), before);
    let mut f = Fixture::new(false);
    let mut value = serde_json::to_value(f.views.recovery(&f.session.editor).unwrap()).unwrap();
    value["next_id"] = (u64::MAX - 1).to_string().into();
    f.views = Views::restore(&serde_json::to_vec(&value).unwrap(), &f.session.editor).unwrap();
    let (p, r) = f.selected();
    let before = f.state();
    assert!(p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap_err()
        .contains("EXHAUSTED"));
    assert_eq!(f.state(), before);
    assert!(p
        .review(&f.source, &f.session.editor, &f.views, " ")
        .is_err());
    assert!(p
        .review(&f.source, &f.session.editor, &f.views, &"界".repeat(400))
        .is_err());
    assert_eq!(f.state(), before);
}

#[test]
fn undo_retires_ids_and_native_undo_cannot_be_resurrected_by_copy_redo() {
    let mut f = Fixture::new(false);
    let stamp = f.views.stamp(&f.session.editor);
    let target = match f
        .views
        .dispatch(
            &mut f.session.editor,
            &stamp,
            NativeAction::Command(Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "New chosen target".into(),
            }),
        )
        .unwrap()
    {
        NativeOutcome::Edit(o) => o.created.unwrap(),
        _ => panic!(),
    };
    let mut p = Plan::new(
        &f.source,
        f.source.declared().primary,
        &f.session.editor,
        &f.views,
    )
    .unwrap();
    p.choose(
        &f.source,
        &f.session.editor,
        &f.views,
        f.row(100),
        Choice::Target(target),
    )
    .unwrap();
    let r = p
        .review(
            &f.source,
            &f.session.editor,
            &f.views,
            "Native history boundary",
        )
        .unwrap();
    p.apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap();
    let high = f.views.document().next_id();
    f.views
        .undo(&f.session.editor, &f.views.stamp(&f.session.editor), false)
        .unwrap();
    let stamp = f.views.stamp(&f.session.editor);
    f.views
        .dispatch(&mut f.session.editor, &stamp, NativeAction::ModelUndo)
        .unwrap();
    assert!(!f.session.editor.snapshot.records.contains_key(&target));
    let before = f.state();
    assert!(f
        .views
        .undo(&f.session.editor, &f.views.stamp(&f.session.editor), true)
        .is_err());
    assert_eq!(f.state(), before);
    assert_eq!(f.views.document().next_id(), high);
    let (p, r) = f.selected();
    let next = p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap();
    assert!(next.get() >= high);
}

#[test]
fn frozen_v1_destination_refuses_v2_and_fresh_pair_round_trips() {
    let mut f = Fixture::new(false);
    let native = f.area.path().join("native.kernspan");
    let job = f.session.begin_save(&native).unwrap();
    assert!(f.session.complete(stage1_io::execute_write(job)));
    let old = f.area.path().join("old-v1.mbse-views.json");
    let mut io = Io::default();
    let job = io.begin(&f.session, &f.views, &old, Kind::Bound).unwrap();
    assert!(io.complete(&f.session, &f.views, io::execute(job)));
    let original = std::fs::read(&old).unwrap();
    let destination = io
        .freeze_destination(&f.session, &f.views, &old, Kind::Bound)
        .unwrap();
    let (p, r) = f.selected();
    let id = p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap();
    assert!(io
        .begin_captured(
            &f.session,
            &f.views,
            f.views.capture(&f.session.editor),
            destination
        )
        .err()
        .unwrap()
        .contains("DESTINATION_FORMAT_CHANGED"));
    assert!(!io.pending());
    assert_eq!(std::fs::read(&old).unwrap(), original);
    assert!(io
        .begin(&f.session, &f.views, &old, Kind::Bound)
        .err()
        .unwrap()
        .contains("FORMAT_SAVE_AS"));
    let path = f.area.path().join("fresh-v2.mbse-views.json");
    let job = io.begin(&f.session, &f.views, &path, Kind::Bound).unwrap();
    assert!(io.complete(&f.session, &f.views, io::execute(job)));
    io.verify_preserved(&f.session, &f.views, Kind::Bound)
        .unwrap();
    let mut fresh = Views::new(&f.session.editor).unwrap();
    Io::default()
        .open(&f.session, &mut fresh, &path, false)
        .unwrap();
    assert_eq!(fresh.history_usage(), (0, 0));
    assert_eq!(
        fresh.document().views[&id].copy_origin,
        f.views.document().views[&id].copy_origin
    );
}

#[test]
fn independent_native_recovery_maps_explicitly_without_old_binding_admission() {
    let mut f = Fixture::new(false);
    let job = f.session.begin_recovery().unwrap();
    assert!(f.session.complete(stage1_io::execute_write(job)));
    let path = f.session.last_recovery().unwrap().to_owned();
    let mut recovered = Session::restore(&f.area.path().join("new-state"), &path).unwrap();
    assert_ne!(
        recovered.editor.snapshot.document_id,
        f.session.editor.snapshot.document_id
    );
    let mut views = Views::new(&recovered.editor).unwrap();
    let mut plan = Plan::new(
        &f.source,
        f.source.declared().primary,
        &recovered.editor,
        &views,
    )
    .unwrap();
    plan.choose(
        &f.source,
        &recovered.editor,
        &views,
        f.row(301),
        Choice::Target(RecordId(301)),
    )
    .unwrap();
    let r = plan
        .review(
            &f.source,
            &recovered.editor,
            &views,
            "Independent recovered presentation",
        )
        .unwrap();
    let id = plan
        .apply_copy(&f.source, &recovered.editor, &mut views, &r)
        .unwrap();
    let native = f.area.path().join("recovered.kernspan");
    let job = recovered.begin_save(&native).unwrap();
    assert!(recovered.complete(stage1_io::execute_write(job)));
    let path = f.area.path().join("recovered.mbse-views.json");
    let mut io = Io::default();
    let job = io.begin(&recovered, &views, &path, Kind::Bound).unwrap();
    assert!(io.complete(&recovered, &views, io::execute(job)));
    assert!(views.document().views[&id].copy_origin.is_some());
    assert_eq!(
        recovered.editor.snapshot.records,
        f.session.editor.snapshot.records
    );
    assert_eq!(
        recovered.editor.snapshot.drafts,
        f.session.editor.snapshot.drafts
    );
}

#[test]
fn combined_provenance_budget_refuses_entire_staged_transaction() {
    let mut f = Fixture::new(false);
    let mut value = serde_json::to_value(f.views.recovery(&f.session.editor).unwrap()).unwrap();
    value["profile"] = v::COPY_RECOVERY_PROFILE.into();
    value["views"]["1"]["copy_origin"] = json!({"profile":"mbse-craft.presentation-copy-origin/1","source_sha256":"a".repeat(64),"source_profile":v::RECOVERY_PROFILE,"source_document_id":"archived","source_sidecar_id":"views:archived","source_view":"1","declared_native_binding":null,"occurrences":(3..1003).map(|id|(id.to_string(),json!({"source_record":"1","expected_kind":"System","outcome":{"state":"Skipped","reason":"x".repeat(1024)}}))).collect::<serde_json::Map<_,_>>(),"edges":{}});
    f.views = Views::restore(&serde_json::to_vec(&value).unwrap(), &f.session.editor).unwrap();
    let mut value = serde_json::to_value(f.source.declared()).unwrap();
    let template = value["occurrences"][f.row(1).get().to_string()].clone();
    let count = value["occurrences"]
        .as_object()
        .unwrap()
        .values()
        .filter(|o| o["view"] == "1")
        .count();
    for i in 0..v::MAX_PER_VIEW - count {
        let id = (1000 + i).to_string();
        let mut row = template.clone();
        row["id"] = id.clone().into();
        value["occurrences"][id] = row;
    }
    value["next_id"] = "3000".into();
    std::fs::write(f.source.path(), serde_json::to_vec(&value).unwrap()).unwrap();
    f.source = Inspection::open(f.source.path()).unwrap();
    let mut plan = Plan::new(
        &f.source,
        f.source.declared().primary,
        &f.session.editor,
        &f.views,
    )
    .unwrap();
    let rows: Vec<_> = plan.choices().keys().copied().collect();
    let chosen = rows[0];
    for id in rows {
        let choice = if id == chosen {
            Choice::Target(f.source.declared().occurrences[&id].record)
        } else {
            Choice::Skip("s".repeat(1024))
        };
        plan.choose(&f.source, &f.session.editor, &f.views, id, choice)
            .unwrap();
    }
    let r = plan
        .review(
            &f.source,
            &f.session.editor,
            &f.views,
            "Over combined provenance budget",
        )
        .unwrap();
    let before = f.state();
    assert!(plan
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap_err()
        .contains("PROVENANCE_BUDGET"));
    assert_eq!(f.state(), before);
}

#[test]
fn detached_source_requires_explicit_match_and_remains_unchanged() {
    let mut f = Fixture::new(false);
    let mut value = serde_json::to_value(f.source.declared()).unwrap();
    let id = f.row(300);
    value["occurrences"][id.get().to_string()]["presence"] =
        json!({"state":"Detached","reason":"Prior source history is untrusted"});
    for edge in value["edges"].as_object_mut().unwrap().values_mut() {
        if edge["source"] == id.get().to_string() || edge["target"] == id.get().to_string() {
            edge["presence"] =
                json!({"state":"Detached","reason":"Historical endpoint is detached"});
        }
    }
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(f.source.path(), &bytes).unwrap();
    f.source = Inspection::open(f.source.path()).unwrap();
    let (p, r) = f.selected();
    let view = p
        .apply_copy(&f.source, &f.session.editor, &mut f.views, &r)
        .unwrap();
    assert!(f
        .views
        .document()
        .occurrences
        .values()
        .any(|o| o.view == view && o.record == RecordId(300) && o.presence.active()));
    assert_eq!(std::fs::read(f.source.path()).unwrap(), bytes);
    assert!(matches!(
        f.source.declared().occurrences[&id].presence,
        v::Presence::Detached { .. }
    ));
}

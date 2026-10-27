use mbse_model::{
    extensions::Extensions,
    stage1::*,
    types::{
        self as t,
        activities::{
            self as a,
            execution::{self as x, Execution, Status},
            forms::{self as f, Form},
        },
        Id,
    },
};
use serde_json::{json, Value};
fn editor() -> Editor {
    let mut s = Snapshot::new(DocumentId("activity:execution-oracle".into()));
    s.extensions = Some(Extensions::default());
    Editor::new(s, "execution:runtime".into()).unwrap()
}
fn run(e: &mut Editor, action: a::Action) {
    e.command(&e.freshness(), Command::Typed(t::Action::Activity(action)))
        .unwrap();
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/parity/p31a/fixtures/control-traces.json"
    ))
    .unwrap()
}
fn graph(name: &str) -> Editor {
    let mut e = editor();
    run(
        &mut e,
        a::Action::Activity {
            id: None,
            system: RecordId(1),
            name: "Fixed control graph".into(),
        },
    );
    for n in oracle()["graphs"][name]["nodes"].as_array().unwrap() {
        assert_eq!(e.snapshot.next_id, n[0].as_u64().unwrap());
        run(
            &mut e,
            a::Action::Node {
                activity: Id(1000),
                id: None,
                name: format!("Node{}", n[0]),
                kind: serde_json::from_value(n[1].clone()).unwrap(),
                partition: None,
            },
        );
    }
    for f in oracle()["graphs"][name]["flows"].as_array().unwrap() {
        assert_eq!(e.snapshot.next_id, f[0].as_u64().unwrap());
        run(
            &mut e,
            a::Action::SetFlowForm(a::FlowForm {
                activity: Id(1000),
                id: None,
                name: format!("Flow{}", f[0]),
                kind: a::FlowKind::Control,
                source: f[1].to_string(),
                target: f[2].to_string(),
                guard: f[3].as_str().unwrap().into(),
            }),
        );
        run(&mut e, a::Action::ApplyFlow);
    }
    e
}
#[test]
fn finite_activity_traces_match_independently_specified_branch_deadlock_cycle_and_cancel_tables() {
    for case in oracle()["cases"].as_array().unwrap() {
        let e = graph(case["graph"].as_str().unwrap());
        let before = e.snapshot.clone();
        let mut live =
            Execution::start(&e, Id(1000), case["budget"].as_u64().unwrap() as usize).unwrap();
        for _ in 0..4097 {
            if live.report().status.terminal() {
                break;
            }
            if case["cancel_after"].as_u64() == Some(live.report().visits.len() as u64) {
                live.cancel(&e);
            } else {
                live.step(&e);
            }
        }
        assert_eq!(
            serde_json::to_value(&live.report().status).unwrap(),
            case["status"],
            "{case}"
        );
        let visits: Vec<_> = live
            .report()
            .visits
            .iter()
            .map(|v| json!([v.node, v.flow]))
            .collect();
        assert_eq!(json!(visits), case["visits"], "{case}");
        assert_eq!(e.snapshot, before);
        assert!(live.is_current(&e));
    }
}
#[test]
fn finite_activity_budget_counts_visits_and_final_at_exact_budget_completes() {
    let e = graph("true_branch");
    for budget in [1, 3, 4, 4096] {
        let mut r = Execution::start(&e, Id(1000), budget).unwrap();
        while !r.report().status.terminal() {
            r.step(&e);
        }
        assert!(r.report().visits.len() <= budget);
        assert_eq!(
            r.report().status,
            if budget < 4 {
                Status::BudgetExhausted
            } else {
                Status::Completed
            }
        );
    }
    assert!(Execution::start(&e, Id(1000), 0)
        .err()
        .unwrap()
        .contains("BUDGET"));
    assert!(Execution::start(&e, Id(1000), 4097)
        .err()
        .unwrap()
        .contains("BUDGET"));
    let e = graph("cycle");
    let mut r = Execution::start(&e, Id(1000), 4096).unwrap();
    while !r.report().status.terminal() {
        r.step(&e);
    }
    assert_eq!(r.report().visits.len(), 4096);
    assert_eq!(r.report().status, Status::BudgetExhausted);
}
#[test]
fn finite_activity_cancellation_and_terminal_steps_are_idempotent_without_model_or_history_writes()
{
    let mut e = graph("cycle");
    let before = e.snapshot.clone();
    let history = (e.can_undo(), e.can_redo());
    let mut r = Execution::start(&e, Id(1000), 1).unwrap();
    r.cancel(&e);
    assert_eq!(r.report().status, Status::Cancelled);
    assert!(r.report().visits.is_empty());
    let cancelled = r.report().clone();
    r.step(&e);
    r.cancel(&e);
    assert_eq!(r.report(), &cancelled);
    assert_eq!(e.snapshot, before);
    assert_eq!((e.can_undo(), e.can_redo()), history);
    e.undo_model(&e.freshness()).unwrap();
    assert!(!r.is_current(&e));
    r.step(&e);
    assert_eq!(r.report(), &cancelled);
}
#[test]
fn finite_activity_source_digest_and_exact_runtime_reject_same_counter_changed_or_reopened_source()
{
    let e = graph("true_branch");
    let mut live = Execution::start(&e, Id(1000), 8).unwrap();
    live.step(&e);
    let initial = live.report().visits.clone();
    let other = Editor::new(e.snapshot.clone(), "other:runtime".into()).unwrap();
    assert!(!live.is_current(&other));
    live.step(&other);
    assert_eq!(live.report().status, Status::StaleSource);
    assert_eq!(live.report().visits, initial);
    let mut live = Execution::start(&e, Id(1000), 8).unwrap();
    let mut changed = e.snapshot.clone();
    changed.records.get_mut(&RecordId(1)).unwrap().name = "Changed without counters".into();
    let changed = Editor::new(changed, e.freshness().instance_id).unwrap();
    assert_eq!(changed.freshness(), e.freshness());
    assert!(!live.is_current(&changed));
    live.cancel(&changed);
    assert_eq!(live.report().status, Status::StaleSource);
    assert!(live.report().visits.is_empty());
}
#[test]
fn finite_activity_new_raw_invalid_guard_stales_execution_without_overwriting_accepted_or_proposal()
{
    let mut e = graph("true_branch");
    let accepted = t::load(&e.snapshot).unwrap();
    let mut live = Execution::start(&e, Id(1000), 8).unwrap();
    run(
        &mut e,
        a::Action::Form(f::Action::Set(Form::Flow {
            form: a::FlowForm {
                activity: Id(1000),
                id: Some(Id(1007)),
                name: "Raw invalid guard".into(),
                kind: a::FlowKind::Control,
                source: "1002".into(),
                target: "1003".into(),
                guard: "x >".into(),
            },
        })),
    );
    let before = e.snapshot.clone();
    live.step(&e);
    assert_eq!(live.report().status, Status::StaleSource);
    assert_eq!(e.snapshot, before);
    assert_eq!(t::load(&e.snapshot).unwrap(), accepted);
    assert_eq!(
        f::load(&e.snapshot).unwrap()["flow:1000:1007"],
        Form::Flow {
            form: a::FlowForm {
                activity: Id(1000),
                id: Some(Id(1007)),
                name: "Raw invalid guard".into(),
                kind: a::FlowKind::Control,
                source: "1002".into(),
                target: "1003".into(),
                guard: "x >".into()
            }
        }
    );
}
#[test]
fn finite_activity_serialized_reports_remain_historical_and_cannot_authorize_resume() {
    let e = graph("true_branch");
    let mut live = Execution::start(&e, Id(1000), 8).unwrap();
    while !live.report().status.terminal() {
        live.step(&e);
    }
    let bytes = serde_json::to_vec(live.report()).unwrap();
    let historical: x::Report = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(&historical, live.report());
    assert_eq!(historical.engine, x::ENGINE);
    let fresh = Execution::start(&e, Id(1000), 8).unwrap();
    assert_eq!(fresh.report().status, Status::Ready);
    assert!(fresh.report().visits.is_empty());
}
#[test]
fn finite_activity_missing_initial_foreign_ids_fanout_and_unsupported_payloads_refuse_without_mutation(
) {
    let e = editor();
    assert!(Execution::start(&e, Id(9999), 8).is_err());
    let mut e = graph("true_branch");
    run(
        &mut e,
        a::Action::Node {
            activity: Id(1000),
            id: Some(Id(1001)),
            name: "No start".into(),
            kind: a::NodeKind::Action,
            partition: None,
        },
    );
    let before = e.snapshot.clone();
    assert!(Execution::start(&e, Id(1000), 8)
        .err()
        .unwrap()
        .contains("INITIAL"));
    assert_eq!(e.snapshot, before);
    let mut e = graph("true_branch");
    run(
        &mut e,
        a::Action::SetFlowForm(a::FlowForm {
            activity: Id(1000),
            id: None,
            name: "Unsupported fork".into(),
            kind: a::FlowKind::Control,
            source: "1003".into(),
            target: "1004".into(),
            guard: "".into(),
        }),
    );
    run(&mut e, a::Action::ApplyFlow);
    assert!(Execution::start(&e, Id(1000), 8)
        .err()
        .unwrap()
        .contains("FORK"));
    let bytes = include_bytes!(
        "../../../docs/parity/p30c/verification/native-04/run/activity-cold-copy.kernspan"
    );
    let root: Value = serde_json::from_slice(bytes).unwrap();
    let s: Snapshot = serde_json::from_value(root["snapshot"].clone()).unwrap();
    let e = Editor::new(s, "activity:payload-refusal".into()).unwrap();
    let before = e.snapshot.clone();
    assert!(Execution::start(&e, Id(1033), 8)
        .err()
        .unwrap()
        .contains("OBJECT_PINS_OR_FLOWS_UNSUPPORTED"));
    assert_eq!(e.snapshot, before);
}

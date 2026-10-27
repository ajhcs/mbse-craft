use mbse_model::{
    extensions::Extensions,
    stage1::*,
    types::{
        self as t, Id,
        state_machines::{
            self as sm, Form,
            execution::{self as x, Execution, Status},
        },
    },
};
use serde_json::{Value, json};
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/parity/p33a/fixtures/state-traces.json"
    ))
    .unwrap()
}
fn run(e: &mut Editor, a: sm::Action) {
    e.command(&e.freshness(), Command::Typed(t::Action::StateMachine(a)))
        .unwrap();
}
fn accept(e: &mut Editor, f: Form) {
    let key = f.key();
    run(e, sm::Action::Set(f));
    run(e, sm::Action::Apply(key));
}
fn graph(name: &str) -> Editor {
    let mut s = Snapshot::new(DocumentId("state:fixed-oracle".into()));
    s.extensions = Some(Extensions::default());
    let mut e = Editor::new(s, "state:runtime".into()).unwrap();
    accept(
        &mut e,
        Form::Machine {
            id: None,
            system: "1".into(),
            name: "Fixed controller".into(),
        },
    );
    for n in oracle()["graphs"][name]["states"].as_array().unwrap() {
        assert_eq!(e.snapshot.next_id, n[0].as_u64().unwrap());
        accept(
            &mut e,
            Form::State {
                machine: Id(1000),
                id: None,
                name: format!("State{}", n[0]),
                state_kind: serde_json::from_value(n[1].clone()).unwrap(),
            },
        );
    }
    for n in oracle()["graphs"][name]["events"].as_array().unwrap() {
        assert_eq!(e.snapshot.next_id, n[0].as_u64().unwrap());
        accept(
            &mut e,
            Form::Event {
                machine: Id(1000),
                id: None,
                name: n[1].as_str().unwrap().into(),
            },
        );
    }
    for n in oracle()["graphs"][name]["transitions"].as_array().unwrap() {
        assert_eq!(e.snapshot.next_id, n[0].as_u64().unwrap());
        accept(
            &mut e,
            Form::Transition {
                machine: Id(1000),
                id: None,
                name: format!("Transition{}", n[0]),
                source: n[1].to_string(),
                target: n[2].to_string(),
                event: if n[3].is_null() {
                    String::new()
                } else {
                    n[3].to_string()
                },
                guard: n[4].as_str().unwrap().into(),
            },
        );
    }
    e
}
fn finish(r: &mut Execution, e: &Editor) {
    for _ in 0..4097 {
        if r.report().status.terminal() {
            return;
        }
        r.step(e);
    }
    assert!(
        r.report().status.terminal(),
        "finite execution did not terminate"
    );
}
#[test]
fn finite_state_traces_match_predeclared_event_conflict_completion_cycle_cancel_and_last_budget_tables()
 {
    for c in oracle()["cases"].as_array().unwrap() {
        let e = graph(c["graph"].as_str().unwrap());
        let before = e.snapshot.clone();
        let history = (e.can_undo(), e.can_redo());
        let mut live = Execution::start(
            &e,
            Id(1000),
            c["budget"].as_u64().unwrap() as usize,
            serde_json::from_value(c["events"].clone()).unwrap(),
        )
        .unwrap();
        for _ in 0..4097 {
            if live.report().status.terminal() {
                break;
            }
            if c["cancel_after"]
                .as_u64()
                .is_some_and(|n| n as usize == live.report().steps.len())
            {
                live.cancel(&e);
            } else {
                live.step(&e);
            }
        }
        assert_eq!(
            serde_json::to_value(&live.report().steps).unwrap(),
            c["steps"],
            "{}",
            c["name"]
        );
        assert_eq!(
            serde_json::to_value(&live.report().status).unwrap(),
            c["status"],
            "{}",
            c["name"]
        );
        assert_eq!(
            live.report().consumed_events,
            c["consumed"].as_u64().unwrap() as usize
        );
        assert_eq!(live.report().current_state.0, c["state"].as_u64().unwrap());
        assert_eq!(e.snapshot, before);
        assert_eq!((e.can_undo(), e.can_redo()), history);
    }
}
#[test]
fn finite_state_exact_event_and_step_budgets_refuse_out_of_bounds_without_source_writes() {
    let e = graph("cycle");
    let before = e.snapshot.clone();
    for budget in [0, 4097] {
        assert!(
            Execution::start(&e, Id(1000), budget, vec![])
                .err()
                .unwrap()
                .contains("BUDGET")
        );
    }
    assert!(
        Execution::start(&e, Id(1000), 1, vec![Id(1005); 4097])
            .err()
            .unwrap()
            .contains("EVENT_LIMIT")
    );
    let mut r = Execution::start(&e, Id(1000), 4096, vec![Id(1005); 4096]).unwrap();
    finish(&mut r, &e);
    assert_eq!(r.report().steps.len(), 4096);
    assert_eq!(r.report().status, Status::BudgetExhausted);
    assert_eq!(r.report().consumed_events, 0);
    assert_eq!(e.snapshot, before);
}
#[test]
fn finite_state_cancel_terminal_idempotence_and_model_history_remain_disjoint() {
    let mut e = graph("true");
    let before = e.snapshot.clone();
    let mut r = Execution::start(&e, Id(1000), 3, vec![Id(1005), Id(1006)]).unwrap();
    r.cancel(&e);
    assert_eq!(r.report().status, Status::Cancelled);
    assert!(r.report().steps.is_empty());
    let cancelled = r.report().clone();
    r.step(&e);
    r.cancel(&e);
    assert_eq!(r.report(), &cancelled);
    assert_eq!(e.snapshot, before);
    e.undo_model(&e.freshness()).unwrap();
    assert!(!r.is_current(&e));
    r.step(&e);
    assert_eq!(r.report(), &cancelled);
    e.redo_model(&e.freshness()).unwrap();
    assert_eq!(t::load(&e.snapshot).unwrap(), t::load(&before).unwrap());
}
#[test]
fn finite_state_same_counter_changed_digest_and_new_runtime_cannot_resume_live_authority() {
    let e = graph("true");
    let mut r = Execution::start(&e, Id(1000), 8, vec![Id(1005)]).unwrap();
    r.step(&e);
    let steps = r.report().steps.clone();
    let other = Editor::new(e.snapshot.clone(), "state:other-runtime".into()).unwrap();
    r.step(&other);
    assert_eq!(r.report().status, Status::StaleSource);
    assert_eq!(r.report().steps, steps);
    let mut changed = e.snapshot.clone();
    changed.records.get_mut(&RecordId(1)).unwrap().name = "Same counters different source".into();
    let changed = Editor::new(changed, e.freshness().instance_id).unwrap();
    assert_eq!(changed.freshness(), e.freshness());
    let mut r = Execution::start(&e, Id(1000), 8, vec![]).unwrap();
    r.cancel(&changed);
    assert_eq!(r.report().status, Status::StaleSource);
    assert!(r.report().steps.is_empty());
}
#[test]
fn finite_state_new_invalid_raw_guard_stales_active_trace_preserving_accepted_and_proposal() {
    let mut e = graph("true");
    let accepted = t::load(&e.snapshot).unwrap();
    let mut r = Execution::start(&e, Id(1000), 8, vec![Id(1005)]).unwrap();
    r.step(&e);
    let f = Form::Transition {
        machine: Id(1000),
        id: Some(Id(1008)),
        name: "Unapplied route".into(),
        source: "1002".into(),
        target: "1003".into(),
        event: "1005".into(),
        guard: "temperature >".into(),
    };
    let key = f.key();
    run(&mut e, sm::Action::Set(f.clone()));
    let before = e.snapshot.clone();
    r.step(&e);
    assert_eq!(r.report().status, Status::StaleSource);
    assert_eq!(r.report().steps.len(), 1);
    assert_eq!(e.snapshot, before);
    assert_eq!(t::load(&e.snapshot).unwrap(), accepted);
    assert_eq!(sm::forms(&e.snapshot).unwrap()[&key], f);
}
#[test]
fn finite_state_reports_are_strict_historical_data_and_fresh_prepare_starts_empty() {
    let e = graph("true");
    let mut r = Execution::start(&e, Id(1000), 3, vec![Id(1005), Id(1006)]).unwrap();
    finish(&mut r, &e);
    let historical: x::Report =
        serde_json::from_slice(&serde_json::to_vec(r.report()).unwrap()).unwrap();
    assert_eq!(&historical, r.report());
    assert_eq!(historical.engine, x::ENGINE);
    assert_eq!(historical.policy, x::POLICY);
    let mut raw = serde_json::to_value(&historical).unwrap();
    raw["resume_authority"] = json!(true);
    assert!(serde_json::from_value::<x::Report>(raw).is_err());
    let fresh = Execution::start(&e, Id(1000), 8, vec![]).unwrap();
    assert_eq!(fresh.report().status, Status::Ready);
    assert!(fresh.report().steps.is_empty());
    assert_eq!(fresh.report().current_state, Id(1001));
}
#[test]
fn finite_state_missing_initial_foreign_event_kind_and_invalid_native_source_refuse_atomically() {
    let mut e = graph("true");
    let before = e.snapshot.clone();
    for event in [Id(1), Id(1001), Id(1007), Id(9999)] {
        assert!(
            Execution::start(&e, Id(1000), 8, vec![event])
                .err()
                .unwrap()
                .contains("EVENT_NOT_OWNED")
        );
    }
    assert!(Execution::start(&e, Id(9999), 8, vec![]).is_err());
    assert_eq!(e.snapshot, before);
    accept(
        &mut e,
        Form::State {
            machine: Id(1000),
            id: Some(Id(1001)),
            name: "No initial".into(),
            state_kind: sm::StateKind::State,
        },
    );
    let before = e.snapshot.clone();
    assert!(
        Execution::start(&e, Id(1000), 8, vec![])
            .err()
            .unwrap()
            .contains("INITIAL")
    );
    assert_eq!(e.snapshot, before);
    let mut e = graph("true");
    e.snapshot.records.remove(&RecordId(1));
    assert!(e.snapshot.validate().is_err());
    assert!(t::load(&e.snapshot).is_ok());
    let before = e.snapshot.clone();
    assert!(Execution::start(&e, Id(1000), 8, vec![]).is_err());
    assert_eq!(e.snapshot, before);
}
#[test]
fn finite_state_execution_preserves_entire_native_authored_snapshot_opaque_four_raw_forms_and_history()
 {
    let raw:Value=serde_json::from_slice(include_bytes!("../../../docs/parity/p32c/verification/native-03/run/state-machine-final-qualified-copy.kernspan")).unwrap();
    let s: Snapshot = serde_json::from_value(raw["snapshot"].clone()).unwrap();
    let e = Editor::new(s, "state:native-preservation".into()).unwrap();
    let before = e.snapshot.clone();
    let mut r = Execution::start(&e, Id(7000), 8, vec![Id(7005)]).unwrap();
    finish(&mut r, &e);
    assert_eq!(
        r.report().status,
        Status::Conflict {
            state: Id(7002),
            event: Some(Id(7005)),
            transitions: vec![Id(7008), Id(7011)]
        }
    );
    assert_eq!(e.snapshot, before);
    let forms = serde_json::to_value(sm::forms(&e.snapshot).unwrap()).unwrap();
    assert_eq!(forms.as_object().unwrap().len(), 12);
    assert_eq!(forms["transition:7000:7008"]["guard"], "temperature >");
    assert_eq!(forms["transition:7000:7009"]["guard"], "else ?");
    assert_eq!(forms["transition:7000:7010"]["target"], "99999");
    assert_eq!(forms["transition:7000:7011"]["event"], "7001");
    assert!(!e.can_undo() && !e.can_redo());
    assert!(
        e.snapshot
            .extensions
            .as_ref()
            .unwrap()
            .namespaces
            .contains_key("fixture.p32c.opaque/1")
    );
}

use super::*;
use mbse_assurance::coverage::CoverageReport;

#[derive(Clone, PartialEq)]
struct Key {
    state: serde_json::Value,
    model: DocumentStamp,
    config: serde_json::Value,
}
struct ResultMessage {
    key: Key,
    result: Result<CoverageReport, String>,
}
#[derive(Default)]
pub(super) struct State {
    pending: Option<Receiver<ResultMessage>>,
    result: Option<ResultMessage>,
    current: bool,
}
fn key(w: &AssuranceWorkspace, e: &Editor) -> Option<Key> {
    Some(Key {
        state: w.close_state()?,
        model: e.calculation_stamp(),
        config: w.config().ok()?,
    })
}
pub(super) fn audit(w: &AssuranceWorkspace) -> serde_json::Value {
    serde_json::json!({"pending":w.coverage.pending.is_some(),"current":w.coverage.current,"report":w.coverage.result.as_ref().and_then(|r|r.result.as_ref().ok()),"error":w.coverage.result.as_ref().and_then(|r|r.result.as_ref().err())})
}
pub(super) fn poll(w: &mut AssuranceWorkspace, ctx: &egui::Context, e: &Editor) {
    if let Some(rx) = &w.coverage.pending {
        match rx.try_recv() {
            Ok(message) => {
                w.coverage.pending = None;
                w.coverage.result = Some(message);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                w.coverage.pending = None;
                w.feedback = "Coverage worker disconnected; model/register retained.".into();
            }
            Err(mpsc::TryRecvError::Empty) => (),
        }
    }
    refresh(w, e);
    if w.coverage.pending.is_some() {
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
pub(super) fn refresh(w: &mut AssuranceWorkspace, e: &Editor) {
    w.coverage.current = w
        .coverage
        .result
        .as_ref()
        .is_some_and(|r| Some(&r.key) == key(w, e).as_ref());
}
fn shortened(s: &str) -> String {
    s.chars().take(256).collect()
}
pub(super) fn show(
    w: &mut AssuranceWorkspace,
    ui: &mut egui::Ui,
    e: &Editor,
    audit: &mut BTreeMap<String, [f32; 4]>,
) {
    // Register Open/evidence completions and new forms may change source after polling.
    refresh(w, e);
    let section=egui::CollapsingHeader::new("Requirement coverage and change impact").id_salt("requirement-coverage").show(ui,|ui|{
        ui.label("Read-only local query. Links declare intent; coverage does not prove fulfillment or verification. Impact lists possible review work; evidence reasons identify known changes or missing provenance.");
        if mark(audit,"coverage:Run",ui.add_enabled(w.coverage.pending.is_none(),egui::Button::new("Run coverage and selected requirement impact"))).clicked(){
            if let Some(key)=key(w,e){let board=w.board.clone();let editor=e.clone();let root=w.selected;let (tx,rx)=mpsc::channel();w.coverage.pending=Some(rx);std::thread::spawn(move||{let result=board.coverage(&editor,&key.config,root);let _=tx.send(ResultMessage{key,result});});}
            else{w.feedback="Coverage requires admitted register and complete context metadata; all work retained.".into();}
        }
        if w.coverage.pending.is_some(){ui.spinner();ui.label("Assessing frozen source; newer edits cannot replace it.");}
        let Some(message)=&w.coverage.result else{ui.label("No query results. Run against the current register/native model.");return;};
        if !w.coverage.current{ui.colored_label(egui::Color32::DARK_RED,"Results are stale after register, native model/raw input or context changes. Run again for current results.");return;}
        let report=match &message.result{Ok(report)=>report.clone(),Err(error)=>{ui.colored_label(egui::Color32::DARK_RED,error);return;}};
        ui.label(format!("{} requirements · {} missing Satisfy · {} missing Verify · {} isolated roots",report.requirements.len(),report.requirements.iter().filter(|r|r.missing_satisfy).count(),report.requirements.iter().filter(|r|r.missing_verify).count(),report.requirements.iter().filter(|r|r.orphan).count()));
        if report.unsupported_count>0{ui.colored_label(egui::Color32::DARK_RED,format!("{} uninterpreted locations: impact/coverage completeness is unproven.",report.unsupported_count));for line in &report.unsupported_locations{ui.label(shortened(line));}}
        if let Some(error)=&report.context_error{ui.colored_label(egui::Color32::DARK_RED,format!("Current evidence context unavailable: {}",shortened(error)));}
        for p in report.detached_traces.iter().take(128){ui.colored_label(egui::Color32::DARK_RED,format!("Detached trace #{}: {}",p.trace,shortened(&p.reason)));}
        for row in report.requirements.iter().take(128){ui.horizontal_wrapped(|ui|{if mark(audit,format!("coverage:select:{}",row.requirement.0),ui.button(format!("{} · local #{}",shortened(&row.identifier),row.requirement.0))).clicked(){w.selected=Some(row.requirement);w.evidence=None;}ui.label(format!("{}{}{}",if row.missing_satisfy{"missing Satisfy; "}else{"Satisfy link present; "},if row.missing_verify{"missing Verify; "}else{"Verify link present; "},if row.orphan{"isolated root"}else{"hierarchy/dependency connected"}));});}
        ui.label(format!("Potential review impact from {}: requirements {:?}; traces {:?}; evidence {:?}. First128 IDs per list; {} requirements, {} traces and {} evidence total. This does not automatically change those records.",report.impact_root.map_or("no selected root".into(),|id|format!("requirement local #{}",id.0)),report.potential_requirements.iter().take(128).map(|id|id.0).collect::<Vec<_>>(),report.potential_traces.iter().take(128).collect::<Vec<_>>(),report.potential_evidence.iter().take(128).map(|id|id.0).collect::<Vec<_>>(),report.potential_requirements.len(),report.potential_traces.len(),report.potential_evidence.len()));
        for row in report.evidence.iter().take(128){ui.label(format!("Evidence #{} / requirement #{}: {}",row.evidence.0,row.requirement.0,if row.reasons.is_empty(){"no freshness/provenance mismatch detected; not a verification decision".into()}else{row.reasons.iter().map(|r|shortened(r)).collect::<Vec<_>>().join("; ")}));}
        ui.label(format!("First128 requirement/evidence/detached rows shown. {} evidence assessed; {} unexamined of{} total. Unknown-location detail capped128. Complete accepted register remains unchanged.",report.evidence.len(),report.evidence_unexamined,report.evidence_total));
    });
    mark(audit, "coverage:section", section.header_response);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (AssuranceWorkspace, Editor) {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/parity/p24/fixtures/native.kernspan"
        ))
        .unwrap();
        let editor = Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
            "p25:render".into(),
        )
        .unwrap();
        let mut w = AssuranceWorkspace::default();
        w.board
            .add_identified_requirement("Source".into(), "R-1".into(), None)
            .unwrap();
        w.board
            .add_identified_requirement("Consumer".into(), "R-2".into(), None)
            .unwrap();
        w.selected = Some(RequirementId(1));
        w.open_window();
        (w, editor)
    }
    #[test]
    fn completed_register_open_invalidates_coverage_before_same_frame_navigation() {
        let (mut w, e) = setup();
        let ctx = egui::Context::default();
        let key = key(&w, &e).unwrap();
        let report = w.board.coverage(&e, &key.config, w.selected).unwrap();
        w.coverage.result = Some(ResultMessage {
            key,
            result: Ok(report),
        });
        let frozen = w.close_state().unwrap();
        let mut loaded = w.board.clone();
        loaded
            .register
            .requirements
            .get_mut(&RequirementId(1))
            .unwrap()
            .statement =
            "Different admitted register with reused local identity and counters".into();
        let expected = loaded.register.clone();
        let path = PathBuf::from(
            "/workspace/build/builderbob/mbse-craft/milestone1/mock-completed-register.json",
        );
        w.path = path.to_string_lossy().into_owned();
        let (tx, rx) = mpsc::channel();
        w.pending = Some(rx);
        tx.send(Completion::Loaded(
            Ok((loaded, "synthetic-valid-worker-hash".into())),
            path,
            frozen,
        ))
        .unwrap();
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2200., 1600.),
                )),
                focused: true,
                ..Default::default()
            },
            |_| w.show(&ctx, &e),
        )
        .drop_without_applying_deltas();
        assert_eq!(w.board.register, expected);
        assert!(
            !w.coverage.current,
            "Loaded register replaced source after pre-frame coverage poll; old navigation must already be stale in this same frame"
        );
        assert!(!w.audit.contains_key("coverage:select:1"));
    }
    #[test]
    fn delayed_coverage_cannot_match_same_revision_but_changed_register_bytes() {
        let (mut w, e) = setup();
        let ctx = egui::Context::default();
        let key = key(&w, &e).unwrap();
        let report = w.board.coverage(&e, &key.config, w.selected).unwrap();
        let (tx, rx) = mpsc::channel();
        w.coverage.pending = Some(rx);
        let old_revision = w.board.register.revision;
        w.board
            .register
            .requirements
            .get_mut(&RequirementId(1))
            .unwrap()
            .rationale = "New accepted payload with same counter".into();
        let before = w.board.register.clone();
        tx.send(ResultMessage {
            key,
            result: Ok(report),
        })
        .unwrap();
        poll(&mut w, &ctx, &e);
        assert!(!w.coverage.current);
        assert_eq!(w.board.register.revision, old_revision);
        assert_eq!(w.board.register, before);
        let mut audit = BTreeMap::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            show(&mut w, ui, &e, &mut audit)
        })
        .drop_without_applying_deltas();
        assert!(!audit.contains_key("coverage:select:1"));
    }
    #[test]
    fn rendered_query_button_and_exact_requirement_navigation_keep_register_and_native_unchanged() {
        let (mut w, e) = setup();
        let native = e.snapshot.clone();
        let before = w.board.register.clone();
        let ctx = egui::Context::default();
        let render = |w: &mut AssuranceWorkspace, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(2200., 1600.),
                    )),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |_| w.show(&ctx, &e),
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut w, vec![]);
        }
        let click = |w: &mut AssuranceWorkspace, key: &str| {
            let r = w.audit[key];
            let pos = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
            for pressed in [true, false] {
                render(
                    w,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            for _ in 0..6 {
                render(w, vec![]);
            }
        };
        click(&mut w, "coverage:section");
        click(&mut w, "coverage:Run");
        for _ in 0..100 {
            render(&mut w, vec![]);
            if w.coverage.current {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(w.coverage.current);
        let report = w.coverage.result.as_ref().unwrap().result.as_ref().unwrap();
        assert_eq!(report.requirements.len(), 2);
        assert!(
            report
                .requirements
                .iter()
                .all(|r| r.orphan && r.missing_satisfy && r.missing_verify)
        );
        click(&mut w, "coverage:select:2");
        assert_eq!(w.selected, Some(RequirementId(2)));
        assert!(w.coverage.current);
        assert_eq!(w.board.register, before);
        assert_eq!(e.snapshot, native);
    }
}

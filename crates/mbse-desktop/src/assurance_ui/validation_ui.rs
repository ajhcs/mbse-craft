use super::*;
use mbse_assurance::validation::{self, Report, RuleFamily, Suite, Target};

/// Desktop form/scope binding around the sealed core location. Never deserialized.
#[derive(Clone, Debug)]
pub(crate) struct Request {
    location: validation::Navigation,
    form: serde_json::Value,
    scope: String,
}
struct Completion {
    report: Result<Report, String>,
    form: serde_json::Value,
    scope: String,
}
pub(super) struct State {
    selected: String,
    pending: Option<Receiver<Completion>>,
    result: Option<Completion>,
    navigation: Option<Request>,
    focus: Option<Request>,
    focus_frames: u8,
    last: Option<(Request, Target, egui::Id)>,
    note: String,
}
impl Default for State {
    fn default() -> Self {
        Self {
            selected: validation::BUILTIN_NAME.into(),
            pending: None,
            result: None,
            navigation: None,
            focus: None,
            focus_frames: 0,
            last: None,
            note: String::new(),
        }
    }
}
fn form(w: &AssuranceWorkspace) -> Option<serde_json::Value> {
    w.close_state()
}
pub(super) fn resolve(
    w: &AssuranceWorkspace,
    request: &Request,
    e: &Editor,
) -> Result<Target, String> {
    if form(w).as_ref() != Some(&request.form) || w.validation.selected != request.scope {
        return Err("VALIDATION_FORM_OR_SCOPE_STALE".into());
    }
    request.location.resolve(&w.board, e)
}
pub(super) fn take(w: &mut AssuranceWorkspace) -> Option<Request> {
    w.validation.navigation.take()
}
pub(super) fn poll(w: &mut AssuranceWorkspace, ctx: &egui::Context) {
    if w.validation.focus.is_some()
        && ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::PointerButton { pressed: true, .. }
                        | egui::Event::Key {
                            key: egui::Key::Tab,
                            pressed: true,
                            ..
                        }
                )
            })
        })
    {
        w.validation.focus = None;
    }
    if let Some(rx) = &w.validation.pending {
        match rx.try_recv() {
            Ok(c) => {
                w.validation.pending = None;
                w.validation.result = Some(c);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                w.validation.pending = None;
                w.validation.note = "Validation worker disconnected; source retained.".into();
            }
            Err(mpsc::TryRecvError::Empty) => (),
        }
    }
    if w.validation.pending.is_some() {
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
fn current(w: &AssuranceWorkspace, e: &Editor, c: &Completion) -> bool {
    c.scope == w.validation.selected
        && form(w).as_ref() == Some(&c.form)
        && c.report
            .as_ref()
            .is_ok_and(|r| r.check(&w.board, e).is_ok())
}
pub(super) fn audit(w: &AssuranceWorkspace, e: &Editor, ctx: &egui::Context) -> serde_json::Value {
    serde_json::json!({"selected_suite":w.validation.selected,"pending":w.validation.pending.is_some(),"current":w.validation.result.as_ref().is_some_and(|c|current(w,e,c)),"report":w.validation.result.as_ref().and_then(|c|c.report.as_ref().ok()),"error":w.validation.result.as_ref().and_then(|c|c.report.as_ref().err()),"note":w.validation.note,"last_focus":w.validation.last.as_ref().map(|(request,target,id)|serde_json::json!({"target":target,"source_current":resolve(w,request,e).is_ok(),"focused":ctx.memory(|m|m.focused())==Some(*id)}))})
}
pub(super) fn focus(
    w: &mut AssuranceWorkspace,
    e: &Editor,
    response: &egui::Response,
    target: Target,
) {
    if w.validation_navigation_blocked() {
        w.validation.focus = None;
        return;
    }
    let Some(request) = w.validation.focus.clone() else {
        return;
    };
    match resolve(w, &request, e) {
        Ok(wanted) if wanted == target => {
            response.request_focus();
            response.scroll_to_me(Some(egui::Align::Center));
            w.validation.last = Some((request, target, response.id));
            w.validation.focus_frames += 1;
            if w.validation.focus_frames >= 3 {
                w.validation.focus = None;
            }
        }
        Ok(_) => (),
        Err(error) => {
            w.validation.focus = None;
            w.validation.note = error;
        }
    }
}
fn short(s: &str) -> String {
    s.chars().take(256).collect()
}
fn target_label(t: &Target) -> String {
    match t {
        Target::Property { property, field } => format!("Property #{} · {field:?}", property.0),
        Target::Usage { usage, field } => format!(
            "Usage {} · {field:?}",
            usage.map_or("new form".into(), |id| format!("#{}", id.0))
        ),
        Target::NativeField(key) => format!("Native #{} · {}", key.record.0, key.field),
        Target::Requirement { requirement, field } => {
            format!("Requirement #{} · {field:?}", requirement.0)
        }
    }
}
pub(super) fn show(
    w: &mut AssuranceWorkspace,
    ui: &mut egui::Ui,
    e: &Editor,
    audit: &mut BTreeMap<String, [f32; 4]>,
) {
    let section=egui::CollapsingHeader::new("Named validation suites and issues").id_salt("named-validation").show(ui,|ui|{
        ui.label("Local working-draft checks. Results identify known issues and excluded scope; they do not grant engineering approval or verification Passed.");
        let names:Vec<_>=std::iter::once(validation::BUILTIN_NAME.to_string()).chain(w.board.register.validation_suites.keys().cloned()).collect();
        let combo=egui::ComboBox::from_id_salt("validation-suite-selection").selected_text(&w.validation.selected).show_ui(ui,|ui|{for name in names{let r=ui.selectable_value(&mut w.validation.selected,name.clone(),&name);mark(audit,format!("validation:suite:{name}"),r);}});mark(audit,"validation:suite",combo.response);
        if mark(audit,"validation:Run",ui.add_enabled(w.validation.pending.is_none(),egui::Button::new("Run selected suite"))).clicked(){
            if let Some(form)=form(w){let board=w.board.clone();let editor=e.clone();let scope=w.validation.selected.clone();let(tx,rx)=mpsc::channel();w.validation.pending=Some(rx);std::thread::spawn(move||{let report=board.run_validation(&scope,&editor);let _=tx.send(Completion{report,form,scope});});}
            else{w.validation.note="Preserve or reduce unadmitted register data before validation.".into();}
        }
        let mut name=w.board.register.drafts.get("validation:name").map(|d|d.text.clone()).unwrap_or_default();let mut rules=w.board.register.drafts.get("validation:rules").map(|d|d.text.clone()).unwrap_or_else(||serde_json::to_string(&Suite::local()).expect("known suite serializes"));let before=(name.clone(),rules.clone());
        ui.horizontal(|ui|{ui.label("Suite name");mark(audit,"validation:name",ui.add(egui::TextEdit::singleline(&mut name).char_limit(128).id(egui::Id::new("validation-name"))));});
        match serde_json::from_str::<Suite>(&rules){Ok(mut suite)=>{let prior=suite.clone();ui.horizontal_wrapped(|ui|{for(rule,label)in[(RuleFamily::PropertyDraft,"Property drafts"),(RuleFamily::PartContainment,"Part containment"),(RuleFamily::CalculationUnits,"Calculation units"),(RuleFamily::RequirementDrafts,"Requirement drafts")]{let mut yes=suite.rules.contains(&rule);if mark(audit,format!("validation:rule:{rule:?}"),ui.checkbox(&mut yes,label)).changed(){if yes{suite.rules.insert(rule);}else{suite.rules.remove(&rule);}}}});if suite!=prior{rules=serde_json::to_string(&suite).expect("known suite serializes");}},Err(error)=>{ui.colored_label(egui::Color32::DARK_RED,format!("Unapplied suite proposal retained: {}",short(&error.to_string())));}}
        ui.collapsing("Advanced suite data",|ui|{ui.label("Known rule families only; opening or editing this data runs no scripts.");mark(audit,"validation:rules-json",ui.add(egui::TextEdit::multiline(&mut rules).char_limit(4096).desired_rows(2).id(egui::Id::new("validation-rules"))));});
        let edited=before!=(name.clone(),rules.clone());if edited{let result=w.board.set_validation_form(name.clone(),rules.clone());w.result(result,"Suite proposal retained independently; save the suite to admit its known rule set.");}
        if mark(audit,"validation:Save suite",ui.add_enabled(!edited,egui::Button::new("Save named suite"))).clicked(){match w.board.apply_validation_form(){Ok(())=>{w.epoch=w.epoch.saturating_add(1);w.validation.selected=name;w.validation.note="Named suite saved in the independent register; run it against current source.".into();},Err(error)=>w.validation.note=error}}
        if w.validation.pending.is_some(){ui.spinner();ui.label("Checking frozen source; newer work is retained.");}
        ui.label(&w.validation.note);
        let Some(c)=&w.validation.result else{ui.label("No report yet. Choose a suite and run it.");return;};
        if c.scope!=w.validation.selected||form(w).as_ref()!=Some(&c.form){ui.colored_label(egui::Color32::DARK_RED,"Report is stale after source/form/scope changes. Run again before navigating.");return;}
        let report=match &c.report{Ok(r)=>r.clone(),Err(error)=>{ui.colored_label(egui::Color32::DARK_RED,error);return;}};
        if let Err(error)=report.check(&w.board,e){ui.colored_label(egui::Color32::DARK_RED,error);return;}
        let frozen_form=c.form.clone();let frozen_scope=c.scope.clone();
        ui.label(format!("Suite {} · {} · {} known issues, {} omitted. Scope limitations {} (first128 retained).",report.suite_name(),report.rules().iter().map(|r|match r{RuleFamily::PropertyDraft=>"property drafts",RuleFamily::PartContainment=>"Part containment",RuleFamily::CalculationUnits=>"calculation units",RuleFamily::RequirementDrafts=>"requirement drafts"}).collect::<Vec<_>>().join(", "),report.issue_total(),report.unreported_issues(),report.limitation_total()));
        for limitation in report.limitations().iter().take(16){ui.label(short(limitation));}
        for issue in report.issues().iter().take(128){ui.horizontal_wrapped(|ui|{ui.label(format!("{} · {}",issue.rule,short(&issue.message)));if mark(audit,format!("validation:issue:{}",issue.id),ui.add_enabled(!w.validation_navigation_blocked(),egui::Button::new(format!("Go to {}",target_label(&issue.target))))).clicked(){match report.navigate(&w.board,e,&issue.id){Ok(location)=>{let request=Request{location,form:frozen_form.clone(),scope:frozen_scope.clone()};match resolve(w,&request,e){Ok(Target::Requirement{requirement,..})=>{w.selected=Some(requirement);w.evidence=None;w.validation.focus=Some(request);w.validation.focus_frames=0;},Ok(_)=>w.validation.navigation=Some(request),Err(error)=>w.validation.note=error}},Err(error)=>w.validation.note=error}}});}
        ui.label("First128 issue rows and16 limitation details displayed. Full retained report has no serialized navigation authority.");
    });
    mark(audit, "validation:section", section.header_response);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (AssuranceWorkspace, Editor) {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/parity/p26a/fixtures/native.kernspan"
        ))
        .unwrap();
        let e = Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
            "p26b:register-render".into(),
        )
        .unwrap();
        let mut w = AssuranceWorkspace::default();
        w.board
            .add_identified_requirement("Root".into(), "R-1".into(), None)
            .unwrap();
        w.board
            .add_identified_requirement("Child".into(), "R-2".into(), Some(RequirementId(1)))
            .unwrap();
        w.board.edit_parent(RequirementId(1), "2".into()).unwrap();
        w.open_window();
        (w, e)
    }
    #[test]
    fn pending_register_io_disables_issue_navigation_without_changing_selection_or_raw() {
        let (mut w, e) = setup();
        let ctx = egui::Context::default();
        w.selected = Some(RequirementId(2));
        let frozen = form(&w).unwrap();
        let report = w
            .board
            .run_validation(validation::BUILTIN_NAME, &e)
            .unwrap();
        w.validation.result = Some(Completion {
            report: Ok(report),
            form: frozen,
            scope: validation::BUILTIN_NAME.into(),
        });
        let (_tx, rx) = mpsc::channel();
        w.pending = Some(rx);
        let before = w.board.register.clone();
        for _ in 0..3 {
            render(&mut w, &e, &ctx, vec![]);
        }
        click(&mut w, &e, &ctx, "validation:section");
        click(
            &mut w,
            &e,
            &ctx,
            "validation:issue:MBSE.REQUIREMENT.DRAFT:requirement1:Parent",
        );
        assert_eq!(w.selected, Some(RequirementId(2)));
        assert!(w.validation.focus.is_none() && w.validation.navigation.is_none());
        assert_eq!(w.board.register, before);
    }
    fn render(
        w: &mut AssuranceWorkspace,
        e: &Editor,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) {
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
            |_| w.show(ctx, e),
        )
        .drop_without_applying_deltas();
    }
    fn click(w: &mut AssuranceWorkspace, e: &Editor, ctx: &egui::Context, key: &str) {
        for _ in 0..3 {
            render(w, e, ctx, vec![]);
        }
        for _ in 0..20 {
            let r = w.audit[key];
            if r[2] > r[0] && r[3] > r[1] && r[1] >= 0. && r[3] <= 1600. {
                break;
            }
            let delta = if r[3] < 200. { 240. } else { -240. };
            render(
                w,
                e,
                ctx,
                vec![
                    egui::Event::PointerMoved(egui::pos2((r[0] + r[2]) / 2., 650.)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        phase: egui::TouchPhase::Move,
                        delta: egui::vec2(0., delta),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        let r = w.audit[key];
        assert!(
            r[2] > r[0] && r[3] > r[1],
            "clipped test widget {key}: {r:?}"
        );
        let pos = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
        for pressed in [true, false] {
            render(
                w,
                e,
                ctx,
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
            render(w, e, ctx, vec![]);
        }
    }
    #[test]
    fn rendered_named_suite_run_and_requirement_issue_focus_preserve_register_raw_and_native() {
        let (mut w, e) = setup();
        let ctx = egui::Context::default();
        for _ in 0..3 {
            render(&mut w, &e, &ctx, vec![]);
        }
        click(&mut w, &e, &ctx, "validation:section");
        click(&mut w, &e, &ctx, "validation:name");
        render(&mut w, &e, &ctx, vec![egui::Event::Text("QA local".into())]);
        click(&mut w, &e, &ctx, "validation:Save suite");
        assert!(w.board.register.validation_suites.contains_key("QA local"));
        let before = w.board.register.clone();
        let native = e.snapshot.clone();
        click(&mut w, &e, &ctx, "validation:Run");
        for _ in 0..100 {
            render(&mut w, &e, &ctx, vec![]);
            if audit(&w, &e, &ctx)["current"] == true {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(audit(&w, &e, &ctx)["current"], true);
        click(
            &mut w,
            &e,
            &ctx,
            "validation:issue:MBSE.REQUIREMENT.DRAFT:requirement1:Parent",
        );
        assert_eq!(w.selected, Some(RequirementId(1)));
        assert_eq!(
            ctx.memory(|m| m.focused()),
            Some(egui::Id::new(("requirement-parent", 1u64)))
        );
        assert_eq!(w.board.register, before);
        assert_eq!(e.snapshot, native);
        assert_eq!(audit(&w, &e, &ctx)["last_focus"]["focused"], true);
    }
    #[test]
    fn desktop_location_rejects_scope_and_unadmitted_form_changes_before_native_dispatch() {
        let (mut w, e) = setup();
        let report = w
            .board
            .run_validation(validation::BUILTIN_NAME, &e)
            .unwrap();
        let request = Request {
            location: report
                .navigate(&w.board, &e, &report.issues()[0].id)
                .unwrap(),
            form: form(&w).unwrap(),
            scope: validation::BUILTIN_NAME.into(),
        };
        assert!(resolve(&w, &request, &e).is_ok());
        w.validation.selected = "Different scope".into();
        assert!(
            resolve(&w, &request, &e)
                .unwrap_err()
                .contains("SCOPE_STALE")
        );
        w.validation.selected = validation::BUILTIN_NAME.into();
        w.statement = "New text retained only in the form".into();
        assert!(report.check(&w.board, &e).is_ok());
        assert!(
            resolve(&w, &request, &e)
                .unwrap_err()
                .contains("FORM_OR_SCOPE_STALE")
        );
        assert_eq!(w.statement, "New text retained only in the form");
    }
    #[test]
    fn completed_open_with_reused_local_ids_invalidates_validation_before_rendering_navigation() {
        let (mut w, e) = setup();
        let ctx = egui::Context::default();
        let frozen = form(&w).unwrap();
        let report = w
            .board
            .run_validation(validation::BUILTIN_NAME, &e)
            .unwrap();
        w.validation.result = Some(Completion {
            report: Ok(report),
            form: frozen.clone(),
            scope: validation::BUILTIN_NAME.into(),
        });
        let mut loaded = w.board.clone();
        loaded
            .register
            .requirements
            .get_mut(&RequirementId(1))
            .unwrap()
            .rationale = "Different admitted bytes with reused counters".into();
        let expected = loaded.register.clone();
        let path = PathBuf::from(
            "/workspace/build/builderbob/mbse-craft/milestone1/mock-p26b-register.json",
        );
        w.path = path.to_string_lossy().into();
        let (tx, rx) = mpsc::channel();
        w.pending = Some(rx);
        tx.send(super::super::Completion::Loaded(
            Ok((loaded, "synthetic-worker-hash".into())),
            path,
            frozen,
        ))
        .unwrap();
        render(&mut w, &e, &ctx, vec![]);
        assert_eq!(w.board.register, expected);
        assert_eq!(audit(&w, &e, &ctx)["current"], false);
        assert!(
            !w.audit
                .contains_key("validation:issue:MBSE.REQUIREMENT.DRAFT:requirement1:Parent")
        );
    }
}

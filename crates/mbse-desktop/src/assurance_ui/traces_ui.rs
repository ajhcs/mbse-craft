use super::*;
use mbse_assurance::{TraceKind, traces};

fn draft(
    w: &mut AssuranceWorkspace,
    ui: &mut egui::Ui,
    audit: &mut BTreeMap<String, [f32; 4]>,
    field: &str,
    label: &str,
) {
    let mut text = w
        .board
        .register
        .drafts
        .get(&format!("trace:{field}"))
        .map(|d| d.text.clone())
        .unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(label);
        let response = ui.add(
            egui::TextEdit::singleline(&mut text)
                .desired_width(650.)
                .char_limit(1024)
                .id(egui::Id::new(("trace-form", field))),
        );
        if mark(audit, format!("traces:{field}"), response).changed() {
            let result = w.board.set_trace_draft(field, text);
            w.result(
                result,
                "Trace proposal retained; only an explicit typed Add admits a dependency.",
            );
        }
    });
}
fn source_label(source: &mbse_assurance::TraceEndpoint) -> String {
    match source {
        mbse_assurance::TraceEndpoint::Requirement(id) => format!("Requirement local #{}", id.0),
        mbse_assurance::TraceEndpoint::TestCase(id) => format!("Documentary test case #{}", id),
        mbse_assurance::TraceEndpoint::Native(n) => {
            format!("{:?} #{} in document {}", n.expected, n.id, n.document)
        }
    }
}
fn value(w: &AssuranceWorkspace, field: &str) -> String {
    w.board
        .register
        .drafts
        .get(&format!("trace:{field}"))
        .map(|d| d.text.clone())
        .unwrap_or_default()
}

pub(super) fn show(
    w: &mut AssuranceWorkspace,
    ui: &mut egui::Ui,
    editor: &Editor,
    target: RequirementId,
    audit: &mut BTreeMap<String, [f32; 4]>,
) {
    let section=egui::CollapsingHeader::new("Typed requirement dependencies").id_salt("requirement-traces").show(ui,|ui|{
        ui.label(format!("Target is selected requirement local #{}. Direction: client/source -> supplier/target.",target.0));
        ui.label("Derive: requirement -> requirement. Refine: requirement/native -> requirement. Satisfy: System/component definition/part -> requirement. Verify: documentary test case -> requirement. Copy: slave requirement -> master requirement, identical read-only statement.");
        ui.label("A dependency declares intent. It never approves a requirement, proves satisfaction, runs a test, or records verification Passed.");
        draft(w,ui,audit,"source","Source (req:ID / native:ID / test:ID)");
        draft(w,ui,audit,"label","Display label (does not select meaning)");
        ui.horizontal_wrapped(|ui|{for kind in [TraceKind::Derive,TraceKind::Refine,TraceKind::Satisfy,TraceKind::Verify,TraceKind::Copy]{if mark(audit,format!("traces:Add {kind:?}"),ui.button(format!("Add {kind:?}"))).clicked(){let result=traces::parse_endpoint(&value(w,"source"),&editor.snapshot).and_then(|source|w.board.add_trace(kind,source,target,value(w,"label"),&editor.snapshot));w.result(result,"Typed dependency added; statuses and native model unchanged.");}}});
        ui.label("New documentary test case: specification only, no execution/proof engine.");
        draft(w,ui,audit,"case_name","Case name");draft(w,ui,audit,"procedure","Procedure");draft(w,ui,audit,"expected","Expected result");
        if mark(audit,"traces:Add test case",ui.button("Add documentary test case")).clicked(){let result=w.board.add_test_case(value(w,"case_name"),value(w,"procedure"),value(w,"expected"));w.result(result,"Test specification added; its test:ID is now the source proposal. No verification decision recorded.");}
        let projection=w.board.trace_projection(&editor.snapshot);
        let links:Vec<_>=w.board.register.traces.values().filter(|t|t.target==target||t.source==mbse_assurance::TraceEndpoint::Requirement(target)).take(128).cloned().collect();
        for trace in links{
            let status=&projection.iter().find(|p|p.trace==trace.id).unwrap();
            ui.label(format!("Trace #{} · {:?} · {:?} -> requirement #{} · {}",trace.id,trace.kind,source_label(&trace.source),trace.target.0,if status.active{"endpoint active"}else{"endpoint detached"}));
            if !status.active{ui.colored_label(egui::Color32::DARK_RED,&status.reason);}
            let mut label=trace.label.clone();
            ui.horizontal(|ui|{ui.label("Display label");if mark(audit,format!("traces:label:{}",trace.id),ui.add(egui::TextEdit::singleline(&mut label).char_limit(128).id(egui::Id::new(("trace-label",trace.id))))).changed(){let result=w.board.rename_trace(trace.id,label);w.result(result,"Display label changed; dependency type and endpoints unchanged.");}if mark(audit,format!("traces:Remove:{}",trace.id),ui.button("Remove dependency")).clicked(){let result=w.board.remove_trace(trace.id);w.result(result,"Dependency removed; Undo remains available. Removing Copy releases its slave statement.");}});
        }
        ui.label("Shows first128 dependencies touching this selection; complete register retained.");
    });
    mark(audit, "traces:section", section.header_response);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rendered_typed_add_wrong_kind_label_history_and_native_preservation() {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/parity/p23/fixtures/profile-tags.kernspan"
        ))
        .unwrap();
        let editor = Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
            "p24:render".into(),
        )
        .unwrap();
        let native = editor.snapshot.clone();
        let mut w = AssuranceWorkspace::default();
        let a = w
            .board
            .add_identified_requirement("Root".into(), "R-1".into(), None)
            .unwrap();
        let c = w
            .board
            .add_identified_requirement("Child".into(), "R-2".into(), Some(a))
            .unwrap();
        w.selected = Some(a);
        w.open_window();
        let ctx = egui::Context::default();
        let render = |w: &mut AssuranceWorkspace, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(2200., 2600.),
                    )),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |_| w.show(&ctx, &editor),
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
        click(&mut w, "traces:section");
        click(&mut w, "traces:source");
        render(&mut w, vec![egui::Event::Text(format!("req:{}", c.0))]);
        click(&mut w, "traces:label");
        render(&mut w, vec![egui::Event::Text("verify".into())]);
        let before = w.board.register.clone();
        click(&mut w, "traces:Add Satisfy");
        assert_eq!(w.board.register, before);
        assert!(w.feedback.contains("ENDPOINT_KIND"));
        click(&mut w, "traces:Add Derive");
        assert_eq!(w.board.register.traces.len(), 1);
        let trace = w.board.register.traces.values().next().unwrap().clone();
        assert_eq!(trace.kind, TraceKind::Derive);
        assert_eq!(trace.label, "verify");
        let after = w.board.register.clone();
        click(&mut w, "assurance:Undo");
        assert!(w.board.register.traces.is_empty());
        click(&mut w, "assurance:Redo");
        assert_eq!(w.board.register, after);
        assert_eq!(editor.snapshot, native);
        assert_eq!(
            w.board.register.requirements[&a].verification,
            VerificationStatus::NotStarted
        );
    }
}

//! Per-view native scope controls; durable proposals live in the existing layout document.
use super::*;
use mbse_model::native_views::{self as nv, OccurrenceId, Stamp, View, ViewId, viewpoints as vp};
#[derive(Clone)]
struct Pending {
    view: ViewId,
    instance: String,
    document: mbse_model::stage1::DocumentId,
    form: vp::Form,
}
struct Rendered {
    view: ViewId,
    stamp: Stamp,
    occurrences: Vec<(OccurrenceId, RecordId)>,
}
#[derive(Default)]
pub(super) struct ViewpointPanel {
    pub open: bool,
    pending: Option<Pending>,
    rendered: Option<Rendered>,
    note: String,
}
impl Workspace {
    pub(super) fn viewpoint_frame_start(&mut self) {
        self.viewpoints.rendered = None;
    }
    pub(super) fn viewpoint_close_blocked(&mut self) -> bool {
        if self.viewpoints.pending.is_some() {
            self.viewpoints.open = true;
            self.viewpoints.note="New form text is retained only in the editor. Reduce it until admitted, copy it, or explicitly discard it before closing.".into();
            true
        } else {
            false
        }
    }
    fn viewpoint_ready(&self) -> bool {
        !self.dock_blocked_without_viewpoint()
            && self.views.views.document().revision() < u64::MAX
            && self
                .views
                .views
                .document()
                .encoded()
                .is_ok_and(|b| b.len().saturating_add(256 * 1024) <= nv::MAX_BYTES)
    }
    fn viewpoint_store_form(
        &mut self,
        view: ViewId,
        stamp: &Stamp,
        form: vp::Form,
    ) -> Result<(), String> {
        let result = if self.dock_blocked_without_viewpoint() {
            Err("VIEWPOINT_BUSY: work retained.".into())
        } else {
            self.views.views.apply(
                &self.session.editor,
                stamp,
                Layout::Viewpoint(view, vp::Action::SetForm(form.clone())),
            )
        };
        match &result {
            Ok(()) => {
                self.viewpoints.pending = None;
                self.viewpoints.note =
                    "Proposal retained in this view. Apply to update the saved viewpoint.".into();
            }
            Err(error) => {
                self.viewpoints.pending = Some(Pending {
                    view,
                    instance: self.session.editor.freshness().instance_id,
                    document: self.session.editor.snapshot.document_id.clone(),
                    form,
                });
                self.viewpoints.note =
                    format!("{error}: new text retained only in this editor; source unchanged.");
            }
        }
        result
    }
    pub(super) fn viewpoint_record_render(
        &mut self,
        view: ViewId,
        occurrences: Vec<(OccurrenceId, RecordId)>,
    ) {
        self.viewpoints.rendered = Some(Rendered {
            view,
            stamp: self.views.views.stamp(&self.session.editor),
            occurrences,
        });
    }
    pub(super) fn viewpoint_summary(&self, ui: &mut egui::Ui, view: &View) {
        if let Some(spec) = &view.viewpoint {
            let p = vp::project(view, &self.session.editor.snapshot);
            ui.label(format!(
                "Viewpoint {} · native card filter {} · System {} · Stakeholder {} · Concern {}",
                spec.name,
                if p.active() { "active" } else { "inactive" },
                spec.system.map_or("all".into(), |id| format!("#{}", id.0)),
                spec.stakeholder
                    .map_or("unset".into(), |id| format!("#{}", id.0)),
                spec.concern
                    .map_or("unset".into(), |id| format!("#{}", id.0))
            ));
            if !p.context_resolved() {
                for s in p.limitations().iter().skip(1) {
                    ui.colored_label(ERROR, s);
                }
            }
        }
    }
    pub(super) fn viewpoint_dialog(&mut self, ctx: &egui::Context) {
        if !self.viewpoints.open {
            return;
        }
        let view_id = self.views.active;
        let Some(view) = self.views.views.document().views.get(&view_id).cloned() else {
            return;
        };
        let stamp = self.views.views.stamp(&self.session.editor);
        let instance = self.session.editor.freshness().instance_id;
        let pending = self.viewpoints.pending.clone();
        let owner_ok = pending.as_ref().is_none_or(|p| {
            p.view == view_id
                && p.instance == instance
                && p.document == self.session.editor.snapshot.document_id
        });
        let mut form = pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| view.viewpoint_form.clone())
            .or_else(|| view.viewpoint.as_ref().map(vp::Form::from_spec))
            .unwrap_or_else(|| vp::Form::new(&self.session.editor));
        let before = form.clone();
        let ready = self.viewpoint_ready() && owner_ok;
        let mut open = true;
        let mut close = false;
        let mut action = None;
        let mut history = None;
        let mut discard = false;
        egui::Window::new("Viewpoint and native card filter").id(egui::Id::new("mbse-viewpoint-filter")).open(&mut open).default_width(900.).default_height(700.).vscroll(true).max_height((ctx.content_rect().height()-80.).max(240.)).show(ctx,|ui|{
   ui.heading(format!("View {} · {}",view_id.get(),view.title));ui.label("Saved scope and proposal are separate. Filters change native card presentation; model fields, manual visibility and independent layouts remain intact.");
   if !owner_ok{ui.colored_label(ERROR,"Retained new text belongs to another view/source; it will not be rebound. Copy it or explicitly discard it.");}
   ui.add_enabled_ui(ready,|ui|{
    for(key,label,value,limit)in[("name","Viewpoint name",&mut form.name,128),("stakeholder","Stakeholder ID (optional)",&mut form.stakeholder,32),("concern","Concern: native Expectation ID (optional)",&mut form.concern,32),("system","System ID (empty = all)",&mut form.system,32),("name-contains","Accepted name contains (literal)",&mut form.name_contains,256)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(value).char_limit(limit).desired_width(430.).id(egui::Id::new(("viewpoint-form",&instance,view_id.get(),key))));self.bound(format!("viewpoint:{key}"),&r);});}
    let r=ui.checkbox(&mut form.enabled,"Enable the saved native-card filter when applied");self.bound("viewpoint:enabled",&r);
    ui.label("Limit native kinds; no checked kinds means all native kinds.");ui.horizontal_wrapped(|ui|{for kind in [RecordKind::System,RecordKind::Mission,RecordKind::Capability,RecordKind::Objective,RecordKind::Measure,RecordKind::Stakeholder,RecordKind::Expectation,RecordKind::Environment,RecordKind::Organization,RecordKind::Actor]{let mut yes=form.kinds.contains(&kind);let r=ui.checkbox(&mut yes,kind.label());self.bound(format!("viewpoint:kind:{kind:?}"),&r);if r.changed(){if yes{form.kinds.insert(kind);}else{form.kinds.remove(&kind);}}}});
   });
   let edited=form!=before;let admitted=self.viewpoints.pending.is_none();
   ui.horizontal_wrapped(|ui|{
    let r=ui.add_enabled(ready&&!edited&&admitted,egui::Button::new("Apply viewpoint proposal"));self.bound("viewpoint:Apply",&r);if r.clicked(){action=Some(vp::Action::Apply);}
    let r=ui.add_enabled(ready&&!edited&&admitted&&view.viewpoint.is_some(),egui::Button::new("Clear saved viewpoint"));self.bound("viewpoint:Clear",&r);if r.clicked(){action=Some(vp::Action::Clear);}
    let r=ui.add_enabled(ready&&!edited&&admitted&&view.viewpoint_form.is_some()&&form.source_document!=self.session.editor.snapshot.document_id,egui::Button::new("Bind proposal to current document"));self.bound("viewpoint:Rebind source",&r);if r.clicked(){action=Some(vp::Action::RebindFormSource);}
    for(redo,label)in[(false,"Layout Undo"),(true,"Layout Redo")]{let r=ui.add_enabled(ready&&!edited&&admitted&&self.views.views.can_layout_history(redo),egui::Button::new(label));self.bound(if redo{"viewpoint:Redo"}else{"viewpoint:Undo"},&r);if r.clicked(){history=Some(redo);}}
   });
   if !ready{ui.label("Finish other work or preserve/reduce layout data before editing this form. Existing proposal text is retained.");}
   if self.viewpoints.pending.is_some(){ui.colored_label(ERROR,"New text is not yet stored in the layout. It remains in this editor.");ui.horizontal(|ui|{let r=ui.button("Copy retained raw proposal");self.bound("viewpoint:Copy unadmitted",&r);if r.clicked(){ctx.copy_text(serde_json::to_string_pretty(&form).expect("form serializes"));}let r=ui.button("Discard unadmitted editor text");self.bound("viewpoint:Discard unadmitted",&r);discard=r.clicked();});}
   ui.label(&self.viewpoints.note);ui.separator();ui.heading("Saved viewpoint");self.viewpoint_summary(ui,&view);
   if view.viewpoint.is_none(){ui.label("No saved viewpoint: native cards remain unfiltered.");}
   let inspected=ui.collapsing("Inspect stakeholder concern and exposed scope",|ui|{
    if let Some(s)=&view.viewpoint{ui.label(format!("Source document: {}",s.source_document.0));for(id,kind,label)in[(s.stakeholder,RecordKind::Stakeholder,"Stakeholder"),(s.concern,RecordKind::Expectation,"Concern (native Expectation)"),(s.system,RecordKind::System,"System scope")]{let text=if s.source_document==self.session.editor.snapshot.document_id{id.and_then(|id|self.session.editor.snapshot.records.get(&id).filter(|r|r.kind==kind).map(|r|format!("#{} · {}",id.0,r.name))).unwrap_or_else(||id.map_or("unset".into(),|id|format!("Detached #{}",id.0)))}else{id.map_or("unset".into(),|id|format!("Foreign reference #{}",id.0))};ui.label(format!("{label}: {text}"));}ui.label(format!("Native kinds: {}",if s.kinds.is_empty(){"all".into()}else{s.kinds.iter().map(|k|k.label()).collect::<Vec<_>>().join(", ")}));ui.label(format!("Literal name substring: {:?}",s.name_contains));}
    ui.label(format!("Proposal source document: {}",form.source_document.0));ui.label("Typed ports/connectors and the complete model/ledger retain their existing scope. These are documentary references, not new SysML model elements or a complete deletion-impact analysis.");
   });self.bound("viewpoint:Inspect scope",&inspected.header_response);
   let r=ui.button("Close viewpoint editor");self.bound("viewpoint:Close",&r);close=r.clicked();
  });
        if discard {
            self.viewpoints.pending = None;
            self.viewpoints.note="Unadmitted editor text explicitly discarded; stored proposal and saved viewpoint unchanged.".into();
        } else if form != before {
            let _ = self.viewpoint_store_form(view_id, &stamp, form);
        } else if let Some(action) = action {
            self.viewpoints.note = match self.views.views.apply(
                &self.session.editor,
                &stamp,
                Layout::Viewpoint(view_id, action),
            ) {
                Ok(()) => {
                    "Saved viewpoint updated; native model and layout geometry unchanged.".into()
                }
                Err(error) => format!("{error}: proposal retained; saved viewpoint unchanged."),
            };
        } else if let Some(redo) = history {
            self.viewpoints.note = match self.views.views.undo(&self.session.editor, &stamp, redo) {
                Ok(true) => "Layout history restored; native model unchanged.".into(),
                Ok(false) => "No layout history in this direction.".into(),
                Err(error) => error,
            };
            self.views.reconcile();
        }
        if close || !open {
            if self.viewpoints.pending.is_some() {
                self.viewpoint_close_blocked();
            } else {
                self.viewpoints.open = false;
            }
        }
    }
    pub(super) fn viewpoint_audit(&self) -> serde_json::Value {
        let view = self.views.views.document().views.get(&self.views.active);
        serde_json::json!({"open":self.viewpoints.open,"active_view":self.views.active,"note":self.viewpoints.note,"saved":view.and_then(|v|v.viewpoint.as_ref()),"form":self.viewpoints.pending.as_ref().map(|p|&p.form).or_else(||view.and_then(|v|v.viewpoint_form.as_ref())),"form_admitted":self.viewpoints.pending.is_none(),"projection":view.map(|v|vp::project(v,&self.session.editor.snapshot)),"rendered":self.viewpoints.rendered.as_ref().map(|r|serde_json::json!({"view":r.view,"current":r.view==self.views.active&&r.stamp==self.views.views.stamp(&self.session.editor),"occurrences":r.occurrences.iter().map(|(o,id)|serde_json::json!({"occurrence":o,"record":id})).collect::<Vec<_>>()}))})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let fixture = serde_json::from_str(include_str!(
            "../../../docs/parity/p29b/fixtures/inspection.json"
        ))
        .unwrap();
        session.editor = Editor::new(
            mbse_model::stage1::Snapshot::from_fixture(
                session.editor.snapshot.document_id.clone(),
                &fixture,
            )
            .unwrap(),
            session.editor.freshness().instance_id,
        )
        .unwrap();
        session
            .editor
            .command(
                &session.editor.freshness(),
                Command::EditText {
                    key: RawKey {
                        record: RecordId(301),
                        field: "criterion".into(),
                        item: None,
                    },
                    text: "9e".into(),
                },
            )
            .unwrap();
        let ctx = egui::Context::default();
        let mut a = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        a.dispatch_workspace(
            crate::command_catalog::Id::Viewpoints,
            crate::command_catalog::Surface::Menu,
            ShortcutTarget::Model,
        );
        (area, a, ctx)
    }
    fn render(a: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>) {
        a.audit.clear();
        a.viewpoint_frame_start();
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2200., 1600.),
                )),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                a.canvas_body(ui);
                a.viewpoint_dialog(ctx);
            },
        )
        .drop_without_applying_deltas();
    }
    fn click(a: &mut Workspace, ctx: &egui::Context, key: &str) {
        for _ in 0..4 {
            render(a, ctx, vec![]);
        }
        let r = a
            .audit
            .get(key)
            .copied()
            .or_else(|| {
                a.views
                    .toolbar_bounds()
                    .find(|(k, _)| k == key)
                    .map(|(_, r)| r)
            })
            .unwrap_or_else(|| panic!("missing {key}"));
        assert!(
            r[2] - r[0] > 5. && r[3] - r[1] >= 12.,
            "clipped {key}: {r:?}"
        );
        let pos = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
        for pressed in [true, false] {
            render(
                a,
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
        for _ in 0..4 {
            render(a, ctx, vec![]);
        }
    }
    fn fill(a: &mut Workspace, ctx: &egui::Context, key: &str, text: &str) {
        click(a, ctx, key);
        render(
            a,
            ctx,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Text(text.into()),
            ],
        );
        for _ in 0..3 {
            render(a, ctx, vec![]);
        }
    }
    fn proposal(a: &mut Workspace) {
        let mut f = vp::Form::new(&a.session.editor);
        f.name = "Quality review".into();
        f.stakeholder = "401".into();
        f.concern = "500".into();
        f.system = "1".into();
        f.kinds.insert(RecordKind::Measure);
        let stamp = a.views.views.stamp(&a.session.editor);
        a.viewpoint_store_form(a.views.active, &stamp, f).unwrap();
    }
    fn ids(a: &Workspace) -> Vec<u64> {
        a.viewpoints
            .rendered
            .as_ref()
            .unwrap()
            .occurrences
            .iter()
            .map(|(_, r)| r.0)
            .collect()
    }
    #[test]
    fn rendered_viewpoint_form_apply_invalid_raw_and_layout_history_preserve_entire_native() {
        let (_area, mut a, ctx) = app();
        let native = a.session.editor.snapshot.clone();
        let old = a.views.views.document().clone();
        for (key, text) in [
            ("name", "Quality review"),
            ("stakeholder", "401"),
            ("concern", "500"),
            ("system", "1"),
        ] {
            fill(&mut a, &ctx, &format!("viewpoint:{key}"), text);
        }
        click(&mut a, &ctx, "viewpoint:kind:Measure");
        click(&mut a, &ctx, "viewpoint:Apply");
        assert_eq!(ids(&a), vec![301]);
        assert!(
            a.viewpoint_audit()["rendered"]["current"]
                .as_bool()
                .unwrap()
        );
        click(&mut a, &ctx, "viewpoint:Undo");
        assert_eq!(ids(&a).len(), 10);
        click(&mut a, &ctx, "viewpoint:Redo");
        assert_eq!(ids(&a), vec![301]);
        fill(&mut a, &ctx, "viewpoint:stakeholder", "401?");
        click(&mut a, &ctx, "viewpoint:Apply");
        assert_eq!(ids(&a), vec![301]);
        let doc = a.views.views.document();
        assert_eq!(
            doc.views[&a.views.active]
                .viewpoint
                .as_ref()
                .unwrap()
                .stakeholder,
            Some(RecordId(401))
        );
        assert_eq!(
            doc.views[&a.views.active]
                .viewpoint_form
                .as_ref()
                .unwrap()
                .stakeholder,
            "401?"
        );
        assert_eq!(doc.occurrences, old.occurrences);
        assert_eq!(
            doc.views[&a.views.active].camera,
            old.views[&a.views.active].camera
        );
        assert_eq!(a.session.editor.snapshot, native);
        assert!(!a.session.editor.can_undo());
    }
    #[test]
    fn rendered_view_switch_keeps_viewpoint_geometry_camera_and_ids_independent() {
        let (_area, mut a, ctx) = app();
        proposal(&mut a);
        click(&mut a, &ctx, "viewpoint:Apply");
        let primary = a.views.active;
        let review = *a
            .views
            .views
            .document()
            .views
            .keys()
            .find(|id| **id != primary)
            .unwrap();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.views
            .views
            .apply(
                &a.session.editor,
                &stamp,
                Layout::Reuse {
                    view: review,
                    record: RecordId(301),
                    geometry: nv::Geometry::at(700., 500.),
                },
            )
            .unwrap();
        let original = a.views.views.document().clone();
        let native = a.session.editor.snapshot.clone();
        a.viewpoints.open = false;
        for _ in 0..3 {
            click(&mut a, &ctx, &format!("view:tab:{}", review.get()));
            assert!(ids(&a).contains(&301));
            assert!(a.views.views.document().views[&review].viewpoint.is_none());
            click(&mut a, &ctx, &format!("view:tab:{}", primary.get()));
            assert_eq!(ids(&a), vec![301]);
        }
        assert_eq!(a.views.views.document().views, original.views);
        assert_eq!(a.views.views.document().occurrences, original.occurrences);
        assert_eq!(a.session.editor.snapshot, native);
    }
    #[test]
    fn failed_form_admission_retains_new_text_and_blocks_close_until_admitted() {
        let (_area, mut a, ctx) = app();
        let before = a.views.views.document().clone();
        let mut f = vp::Form::new(&a.session.editor);
        f.name = "x".repeat(4097);
        let stamp = a.views.views.stamp(&a.session.editor);
        assert!(
            a.viewpoint_store_form(a.views.active, &stamp, f.clone())
                .is_err()
        );
        assert_eq!(a.viewpoints.pending.as_ref().unwrap().form, f);
        assert!(a.viewpoint_close_blocked());
        assert_eq!(a.views.views.document(), &before);
        f.name = "retained correction".into();
        a.viewpoint_store_form(a.views.active, &stamp, f).unwrap();
        assert!(!a.viewpoint_close_blocked());
        render(&mut a, &ctx, vec![]);
        assert!(a.viewpoint_audit()["form_admitted"].as_bool().unwrap());
    }
    #[test]
    fn rendered_fit_uses_current_filtered_cards_and_preserves_native_and_geometry() {
        let (_area, mut a, ctx) = app();
        let far = a
            .views
            .views
            .document()
            .occurrences
            .values()
            .find(|o| o.record == RecordId(401) && o.view == a.views.active)
            .unwrap()
            .id;
        let stamp = a.views.views.stamp(&a.session.editor);
        a.views
            .views
            .apply(
                &a.session.editor,
                &stamp,
                Layout::Move(far, nv::Geometry::at(90000., 90000.)),
            )
            .unwrap();
        a.viewpoints.open = false;
        click(&mut a, &ctx, "camera:Fit");
        let old_zoom = a.views.views.document().views[&a.views.active].camera.zoom;
        let geometry = a.views.views.document().occurrences.clone();
        let native = a.session.editor.snapshot.clone();
        a.viewpoints.open = true;
        proposal(&mut a);
        click(&mut a, &ctx, "viewpoint:Apply");
        a.viewpoints.open = false;
        click(&mut a, &ctx, "camera:Fit");
        assert!(a.views.views.document().views[&a.views.active].camera.zoom > old_zoom);
        assert_eq!(ids(&a), vec![301]);
        assert_eq!(a.views.views.document().occurrences, geometry);
        assert_eq!(a.session.editor.snapshot, native);
    }
    #[test]
    fn foreign_saved_scope_is_disclosed_and_actual_cards_remain_unfiltered() {
        let (_area, mut a, ctx) = app();
        proposal(&mut a);
        click(&mut a, &ctx, "viewpoint:Apply");
        let native = a.session.editor.snapshot.clone();
        let mut doc = a.views.views.recovery(&a.session.editor).unwrap();
        doc.views
            .get_mut(&a.views.active)
            .unwrap()
            .viewpoint
            .as_mut()
            .unwrap()
            .source_document = mbse_model::stage1::DocumentId("foreign-source".into());
        a.views.views = nv::Views::restore(&doc.encoded().unwrap(), &a.session.editor).unwrap();
        render(&mut a, &ctx, vec![]);
        assert_eq!(ids(&a).len(), 10);
        let audit = a.viewpoint_audit();
        assert!(!audit["projection"]["active"].as_bool().unwrap());
        assert_eq!(a.session.editor.snapshot, native);
    }
}

//! One native graph matrix; cell edits use the existing guarded model/view dispatcher.
use super::*;
use mbse_model::relation_matrix::{self as matrix, Kind, Query, Report};
#[derive(Default)]
pub(super) struct MatrixPanel {
    pub open: bool,
    query: Query,
    report: Option<Report>,
    note: String,
}
impl Workspace {
    pub(super) fn matrix_refresh(&mut self) {
        match matrix::query(&self.session.editor, self.matrix.query) {
            Ok(report) => {
                self.matrix.query = report.query();
                self.matrix.report = Some(report);
            }
            Err(error) => {
                self.matrix.report = None;
                self.matrix.note = error;
            }
        }
    }
    fn matrix_apply(&mut self, edit: matrix::Edit) -> Result<(), String> {
        if self.dock_blocked_without_matrix() {
            return Err("MATRIX_BUSY: finish writes/gestures or close another auxiliary editor; source retained.".into());
        }
        let command = edit.resolve(&self.session.editor)?;
        let stamp = self.views.views.stamp(&self.session.editor);
        self.views.views.dispatch(
            &mut self.session.editor,
            &stamp,
            mbse_model::native_views::NativeAction::Command(command),
        )?;
        self.matrix.note =
            "Cell updated by the native relation command. Model Undo reverses the complete edit."
                .into();
        self.matrix_refresh();
        Ok(())
    }
    pub(super) fn matrix_dialog(&mut self, ctx: &egui::Context) {
        if !self.matrix.open {
            return;
        }
        if self.matrix.report.is_none() {
            self.matrix_refresh();
        }
        let mut open = true;
        let mut close = false;
        let mut refresh = false;
        let mut edit = None;
        let mut select = None;
        let mut history = None;
        egui::Window::new("Native relation matrix").id(egui::Id::new("mbse-native-relation-matrix")).open(&mut open).default_width(1050.).default_height(650.).vscroll(true).max_height((ctx.content_rect().height()-80.).max(240.)).show(ctx,|ui|{
   ui.label("Rows are sources; columns are targets. Checked cells are current authored graph links. Wrong-kind/cross-System cells are disabled. No geometry or labels create a relation.");
   let before=self.matrix.query;let q=&mut self.matrix.query;
   let combo=egui::ComboBox::from_id_salt("matrix-kind").selected_text(q.kind.label()).show_ui(ui,|ui|{for kind in [Kind::ContributesTo,Kind::Subject,Kind::ForObjective,Kind::Scope,Kind::CalculationInput]{let r=ui.selectable_value(&mut q.kind,kind,kind.label());self.audit.insert(format!("matrix:kind:{kind:?}"),[r.rect.min.x,r.rect.min.y,r.rect.max.x,r.rect.max.y]);}});self.bound("matrix:kind",&combo.response);
   ui.label("Subject and Measure-to-Objective cells replace the previous single link. A Measure's required Objective cannot be cleared; scope/calculation dependencies are read-only here.");
   let ready=!self.dock_blocked_without_matrix();
   ui.horizontal(|ui|{for(redo,label)in[(false,"Undo model"),(true,"Redo model")]{let enabled=ready&&if redo{self.session.editor.can_redo()}else{self.session.editor.can_undo()};let r=ui.add_enabled(enabled,egui::Button::new(label));self.bound(if redo{"matrix:Redo"}else{"matrix:Undo"},&r);if r.clicked(){history=Some(redo);}}let r=ui.button("Refresh matrix");self.bound("matrix:Refresh",&r);refresh|=r.clicked();let r=ui.button("Close matrix");self.bound("matrix:Close",&r);close=r.clicked();});
   let Some(report)=self.matrix.report.clone()else{ui.colored_label(ERROR,&self.matrix.note);return;};
   ui.horizontal(|ui|{let r=ui.add_enabled(q_offset(&self.matrix.query,true)>0,egui::Button::new("Previous rows"));self.bound("matrix:Previous rows",&r);if r.clicked(){self.matrix.query.row_offset=self.matrix.query.row_offset.saturating_sub(matrix::PAGE);}let r=ui.add_enabled(self.matrix.query.row_offset.saturating_add(matrix::PAGE)<report.total(),egui::Button::new("Next rows"));self.bound("matrix:Next rows",&r);if r.clicked(){self.matrix.query.row_offset=self.matrix.query.row_offset.saturating_add(matrix::PAGE);}let r=ui.add_enabled(self.matrix.query.column_offset>0,egui::Button::new("Previous columns"));self.bound("matrix:Previous columns",&r);if r.clicked(){self.matrix.query.column_offset=self.matrix.query.column_offset.saturating_sub(matrix::PAGE);}let r=ui.add_enabled(self.matrix.query.column_offset.saturating_add(matrix::PAGE)<report.total(),egui::Button::new("Next columns"));self.bound("matrix:Next columns",&r);if r.clicked(){self.matrix.query.column_offset=self.matrix.query.column_offset.saturating_add(matrix::PAGE);}});
   refresh|=before!=self.matrix.query;if refresh{return;}
   ui.label(format!("{} native records · rows {}..{} · columns {}..{} · at most32 by32 cells. Other typed/profile/requirements relations are outside this native graph matrix.",report.total(),self.matrix.query.row_offset,self.matrix.query.row_offset+report.rows().len(),self.matrix.query.column_offset,self.matrix.query.column_offset+report.columns().len()));
   if let Err(error)=report.check(&self.session.editor){ui.colored_label(ERROR,format!("{error}: refresh before editing or navigating."));return;}
   egui::ScrollArea::both().id_salt("matrix-grid-scroll").max_height(420.).show(ui,|ui|{egui::Grid::new("native-relation-grid").striped(true).show(ui,|ui|{
    ui.label("Source -> Target");for a in report.columns(){let r=ui.button(format!("#{}",a.id.0)).on_hover_text(format!("{} #{} · {}",a.kind.label(),a.id.0,a.label));self.bound(format!("matrix:column:{}",a.id.0),&r);if r.clicked(){select=Some(a.id);}}ui.end_row();
    for a in report.rows(){let r=ui.button(format!("{} #{} · {}",a.kind.label(),a.id.0,a.label));self.bound(format!("matrix:row:{}",a.id.0),&r);if r.clicked(){select=Some(a.id);}for b in report.columns(){let c=report.cells().iter().find(|c|c.source==a.id&&c.target==b.id).expect("complete matrix page");let r=ui.add_enabled(ready&&c.editable,egui::Button::new(if c.present{"[x]"}else{"[ ]"})).on_hover_text(c.reason.as_deref().unwrap_or("Edit this exact native relation cell"));self.bound(format!("matrix:cell:{}:{}",a.id.0,b.id.0),&r);if r.clicked(){match report.edit(&self.session.editor,a.id,b.id){Ok(proof)=>edit=Some(proof),Err(error)=>self.matrix.note=error}}}ui.end_row();}
   });});ui.label(&self.matrix.note);
  });
        if let Some(redo) = history {
            if self.dock_blocked_without_matrix() {
                self.matrix.note = "MATRIX_BUSY: history refused; source retained.".into();
            } else {
                match self.typed_model_history(redo) {
                    Ok(_) => {
                        self.matrix.note =
                            "Native model history restored; retained drafts and views preserved."
                                .into();
                        self.matrix_refresh();
                    }
                    Err(error) => self.matrix.note = error,
                }
            }
        }
        if let Some(edit) = edit {
            if let Err(error) = self.matrix_apply(edit) {
                self.matrix.note = error;
            }
        }
        if let Some(id) = select {
            if !self.dock_blocked_without_matrix() {
                match self
                    .matrix
                    .report
                    .as_ref()
                    .ok_or("MATRIX_REPORT_MISSING")
                    .and_then(|r| {
                        r.axis(&self.session.editor, id)
                            .map_err(|_| "MATRIX_AXIS_STALE")
                    }) {
                    Ok(id) => self.select(id),
                    Err(error) => self.matrix.note = error.into(),
                }
            }
        }
        if refresh {
            self.matrix_refresh();
        }
        if close || !open {
            self.matrix.open = false;
        }
    }
    pub(super) fn matrix_audit(&self) -> serde_json::Value {
        serde_json::json!({"open":self.matrix.open,"query":self.matrix.query,"note":self.matrix.note,"current":self.matrix.report.as_ref().is_some_and(|r|r.check(&self.session.editor).is_ok()),"report":self.matrix.report,"graph_edges":stage1_graph::derive_graph(&self.session.editor.snapshot).ok().map(|g|g.edges.into_iter().map(|e|serde_json::json!({"role":format!("{:?}",e.id.role),"source":e.id.source,"target":e.id.target})).collect::<Vec<_>>())})
    }
}
fn q_offset(q: &Query, rows: bool) -> usize {
    if rows { q.row_offset } else { q.column_offset }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p28/fixtures/native.kernspan"
        ))
        .unwrap();
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
            session.editor.freshness().instance_id,
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        app.dispatch_workspace(
            crate::command_catalog::Id::Matrix,
            crate::command_catalog::Surface::Menu,
            ShortcutTarget::Model,
        );
        (area, app, ctx)
    }
    fn render(a: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>) {
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
            |_| a.matrix_dialog(ctx),
        )
        .drop_without_applying_deltas();
    }
    fn click(a: &mut Workspace, ctx: &egui::Context, key: &str) {
        for _ in 0..3 {
            render(a, ctx, vec![]);
        }
        let r = a.audit[key];
        assert!(r[2] > r[0] && r[3] - r[1] >= 12., "clipped {key}: {r:?}");
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
        for _ in 0..3 {
            render(a, ctx, vec![]);
        }
    }
    fn present(a: &Workspace) -> bool {
        a.matrix
            .report
            .as_ref()
            .unwrap()
            .cells()
            .iter()
            .find(|c| c.source == RecordId(1001) && c.target == RecordId(1000))
            .unwrap()
            .present
    }
    #[test]
    fn rendered_matrix_cell_and_graph_agree_through_single_native_undo_redo_preserving_raw_and_views()
     {
        let (_area, mut a, ctx) = app();
        let records = a.session.editor.snapshot.records.clone();
        let raw = a.session.editor.snapshot.drafts.clone();
        let ext = a.session.editor.snapshot.extensions.clone();
        let views = a.views.views.document().clone();
        assert!(present(&a));
        click(&mut a, &ctx, "matrix:cell:1001:1000");
        assert!(!present(&a));
        assert!(
            !stage1_graph::derive_graph(&a.session.editor.snapshot)
                .unwrap()
                .edges
                .iter()
                .any(|x| x.id.role == stage1_graph::RelationRole::ContributesTo)
        );
        click(&mut a, &ctx, "matrix:Undo");
        assert!(present(&a));
        assert_eq!(a.session.editor.snapshot.records, records);
        click(&mut a, &ctx, "matrix:Redo");
        assert!(!present(&a));
        assert_eq!(a.session.editor.snapshot.drafts, raw);
        assert_eq!(a.session.editor.snapshot.extensions, ext);
        let after = a.views.views.document();
        assert_eq!(after.views, views.views);
        assert_eq!(after.occurrences, views.occurrences);
        assert_eq!(after.edges, views.edges);
        assert_eq!(after.typed_ports, views.typed_ports);
        assert_eq!(after.connector_edges, views.connector_edges);
    }
    #[test]
    fn rendered_wrong_kind_cell_is_disabled_and_busy_or_stale_queued_edit_preserves_all_work() {
        let (_area, mut a, ctx) = app();
        let before = a.session.editor.snapshot.clone();
        click(&mut a, &ctx, "matrix:cell:1000:1001");
        assert_eq!(a.session.editor.snapshot, before);
        let p = a
            .matrix
            .report
            .as_ref()
            .unwrap()
            .edit(&a.session.editor, RecordId(1001), RecordId(1000))
            .unwrap();
        a.settings_window = true;
        assert!(
            a.matrix_apply(p.clone())
                .unwrap_err()
                .contains("MATRIX_BUSY")
        );
        assert_eq!(a.session.editor.snapshot, before);
        a.settings_window = false;
        a.session
            .editor
            .snapshot
            .records
            .get_mut(&RecordId(1001))
            .unwrap()
            .name = "Changed native bytes same counters".into();
        let current = a.session.editor.snapshot.clone();
        assert_eq!(a.matrix_apply(p).unwrap_err(), "MATRIX_SOURCE_STALE");
        assert_eq!(a.session.editor.snapshot, current);
        render(&mut a, &ctx, vec![]);
        assert!(!a.matrix_audit()["current"].as_bool().unwrap());
    }
    #[test]
    fn matrix_command_uses_shared_catalog_scope_and_axis_navigation_preserves_source() {
        let (_area, mut a, ctx) = app();
        let before = a.session.editor.snapshot.clone();
        assert!(
            a.command_reason(crate::command_catalog::Id::Save, &ShortcutTarget::Model)
                .unwrap()
                .contains("Relation matrix")
        );
        assert!(a.dock_blocked());
        click(&mut a, &ctx, "matrix:row:1002");
        assert_eq!(a.session.editor.selected, Some(RecordId(1002)));
        assert_eq!(a.session.editor.snapshot, before);
        click(&mut a, &ctx, "matrix:Close");
        assert!(!a.matrix.open);
        a.settings_window = true;
        a.dispatch_workspace(
            crate::command_catalog::Id::Matrix,
            crate::command_catalog::Surface::Keyboard,
            ShortcutTarget::Widget,
        );
        assert!(!a.matrix.open);
        assert_eq!(a.session.editor.snapshot, before);
    }
}

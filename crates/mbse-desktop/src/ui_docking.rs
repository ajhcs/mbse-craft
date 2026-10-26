//! Dock bodies borrow the one live workspace; moves never reconstruct content.
use super::*;
use crate::docking::{Panel, WorkspaceDock};
use craft_ui::docking::{Action, DockArea, DockStyle, Placement, Zone};
impl Workspace {
    fn dock_id() -> egui::Id {
        egui::Id::new("mbse-owned-workspace-docking")
    }
    pub(super) fn docking_chrome_focused(&self) -> bool {
        DockArea::new(Self::dock_id()).chrome_has_focus(&self.context, &self.docking.layout)
    }
    fn dock_blocked(&self) -> bool {
        self.views.occurrence_navigation_blocked(&self.session)
            || self.pending.is_some()
            || self.creation.is_some()
            || self.context_creation.is_some()
            || self.shortcuts.open
            || self.settings_window
            || self.views.inspection_visible()
            || self.assurance.is_visible()
            || self.exchange.is_visible()
            || self.recovery_window
            || self.search_open
            || self.impact_open
            || self.usages.is_some()
            || self.relation.is_some()
            || self.popout.is_some()
    }
    pub(super) fn apply_dock(
        &mut self,
        expected: u64,
        actions: Vec<Action<Panel>>,
    ) -> Result<(), String> {
        if self.dock_blocked() {
            return Err("DOCK_BUSY: finish the gesture/write or close the auxiliary editor first; all panels retained.".into());
        }
        self.docking.apply(expected, actions)
    }
    pub(super) fn dock_surface(&mut self, root: &mut egui::Ui) {
        self.views.clear_toolbar_bounds();
        self.docking.body_ids.clear();
        let layout = self.docking.layout.clone();
        let revision = self.docking.revision;
        egui::CentralPanel::default().show(root, |ui| {
            let style = DockStyle::from_ui(ui);
            let out = DockArea::new(Self::dock_id()).show(
                ui,
                &layout,
                &style,
                |p| p.label().into(),
                |p| p.permissions(),
                |ui, p| {
                    self.docking
                        .body_ids
                        .insert(p.key().into(), format!("{:?}", ui.id()));
                    let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                    self.audit.insert(
                        format!("dock:body:{}", p.key()),
                        [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
                    );
                    match p {
                        Panel::Browser => self.browser_body(ui),
                        Panel::Inspector => self.inspector_body(ui),
                        Panel::Canvas => self.canvas_body(ui),
                    }
                },
            );
            if let Some(error) = out.error {
                self.docking.note = format!("Dock renderer refused: {error:?}; work retained.");
            }
            if !out.actions.is_empty() {
                if let Err(error) = self.apply_dock(revision, out.actions) {
                    self.docking.note = error;
                }
            }
            for panel in Panel::ALL {
                if let Some(response) = ui.ctx().read_response(Self::dock_id().with(("tab", panel)))
                {
                    self.bound(format!("dock:tab:{}", panel.key()), &response);
                }
            }
        });
    }
    pub(super) fn docking_dialog(&mut self, ctx: &egui::Context) {
        if !self.docking.window {
            return;
        }
        let mut open = true;
        let mut close = false;
        let mut action = None;
        let revision = self.docking.revision;
        egui::Window::new("Panel layout").id(egui::Id::new("workspace-panel-controls")).open(&mut open).default_width(610.).show(ctx,|ui|{
            ui.label("Drag panel tabs to move, split or float. Floating panels stay inside this application viewport. Panel contents and unfinished native text remain owned by this workspace.");
            for panel in Panel::ALL{let anchor=if panel==Panel::Canvas{Panel::Browser}else{Panel::Canvas};let tab_anchor=if panel==Panel::Inspector{Panel::Browser}else{Panel::Inspector};
                ui.group(|ui|{ui.label(panel.label());ui.horizontal_wrapped(|ui|{
                    let r=ui.button("Float");self.bound(format!("dock:float:{}",panel.key()),&r);if r.clicked(){let screen=ctx.content_rect();action=Some(Action::Float{panel,rect:[screen.left()+60.,screen.top()+160.,screen.width().min(540.).max(200.),(screen.height()-240.).clamp(200.,850.)]});}
                    let r=ui.button(format!("Tab with {}",tab_anchor.label()));self.bound(format!("dock:tab-with:{}",panel.key()),&r);if r.clicked(){action=Some(Action::Move{panel,anchor:tab_anchor,placement:Placement::Tab{before:None}});}
                    let r=ui.button(format!("Left of {}",anchor.label()));self.bound(format!("dock:left:{}",panel.key()),&r);if r.clicked(){action=Some(Action::Move{panel,anchor,placement:Placement::Split(Zone::Left)});}
                });});
            }
            ui.label(&self.docking.note);let r=ui.button("Close controls");self.bound("dock:close-controls",&r);close=r.clicked();
        });
        self.docking.window = open && !close;
        if let Some(action) = action {
            if let Err(error) = self.apply_dock(revision, vec![action]) {
                self.docking.note = error;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let snap = mbse_model::stage1::Snapshot::from_fixture(
            session.editor.snapshot.document_id.clone(),
            &serde_json::from_str(include_str!(
                "../../mbse-model/fixtures/stage1/inspection.json"
            ))
            .unwrap(),
        )
        .unwrap();
        session.editor =
            mbse_model::stage1::Editor::new(snap, session.editor.freshness().instance_id).unwrap();
        session.editor.select(RecordId(301)).unwrap();
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        app.command(Command::EditText {
            key: RawKey {
                record: RecordId(301),
                field: "criterion".into(),
                item: None,
            },
            text: "9e".into(),
        });
        (area, app, ctx)
    }
    fn retained(a: &Workspace) -> (String, String, Option<RecordId>, (usize, usize), String) {
        (
            a.session.editor.snapshot.digest(),
            a.views.views.document().digest().unwrap(),
            a.session.editor.selected,
            a.views.views.history_usage(),
            a.save_path.clone(),
        )
    }
    fn render(a: &mut Workspace, ctx: &egui::Context) {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2200., 1600.),
                )),
                focused: true,
                ..Default::default()
            },
            |ui| a.dock_surface(ui),
        )
        .drop_without_applying_deltas();
    }
    #[test]
    fn panel_moves_never_reconstruct_or_mutate_native_views_raw_selection_or_save_path() {
        let (_area, mut a, _) = app();
        let before = retained(&a);
        for action in [
            Action::Float {
                panel: Panel::Inspector,
                rect: [20., 80., 400., 800.],
            },
            Action::Move {
                panel: Panel::Inspector,
                anchor: Panel::Browser,
                placement: Placement::Tab { before: None },
            },
            Action::Move {
                panel: Panel::Inspector,
                anchor: Panel::Canvas,
                placement: Placement::Split(Zone::Right),
            },
        ] {
            a.apply_dock(a.docking.revision, vec![action]).unwrap();
            assert_eq!(retained(&a), before);
            assert_eq!(
                a.session.editor.display_text(&RawKey {
                    record: RecordId(301),
                    field: "criterion".into(),
                    item: None
                }),
                "9e"
            );
        }
    }
    #[test]
    fn real_body_and_native_field_widget_ids_and_focus_survive_floating() {
        let (_area, mut a, ctx) = app();
        render(&mut a, &ctx);
        let body = a.docking.body_ids["inspector"].clone();
        let key = RawKey {
            record: RecordId(301),
            field: "criterion".into(),
            item: None,
        };
        let field = *a.model_fields.iter().find(|(_, k)| **k == key).unwrap().0;
        ctx.memory_mut(|m| m.request_focus(field));
        let before = retained(&a);
        a.apply_dock(
            a.docking.revision,
            vec![Action::Float {
                panel: Panel::Inspector,
                rect: [60., 80., 420., 1000.],
            }],
        )
        .unwrap();
        render(&mut a, &ctx);
        assert_eq!(a.docking.body_ids["inspector"], body);
        assert_eq!(a.model_fields[&field], key);
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert_eq!(retained(&a), before);
    }
    #[test]
    fn busy_or_modal_dock_refusal_retains_panel_layout_and_the_owning_native_ticket() {
        let (_area, mut a, _) = app();
        let before = retained(&a);
        let layout = a.docking.layout.clone();
        let job = a.session.begin_recovery().unwrap();
        let action = || Action::Float {
            panel: Panel::Inspector,
            rect: [20., 80., 400., 800.],
        };
        assert!(a
            .apply_dock(0, vec![action()])
            .unwrap_err()
            .contains("BUSY"));
        assert!(a.session.pending());
        assert_eq!(a.docking.layout, layout);
        assert_eq!(retained(&a), before);
        assert!(a
            .session
            .failed_start(job.start_token(), "Unstarted P11 fixture cleanup".into()));
        a.shortcuts.open = true;
        assert!(a.apply_dock(0, vec![action()]).is_err());
        assert_eq!(a.docking.layout, layout);
    }
    #[test]
    fn invalid_pointer_targets_or_canceled_proposals_have_no_mutation_authority() {
        use craft_ui::docking::{resolve_drop, DropTarget};
        let (_area, a, _) = app();
        let before = retained(&a);
        let layout = a.docking.layout.clone();
        let target = DropTarget {
            anchor: Panel::Browser,
            rect: egui::Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(200., 200.)),
            tab_strip: None,
            tabs: vec![],
            allow_splits: false,
            accepts_tabs: false,
        };
        assert!(resolve_drop(&Panel::Inspector, egui::pos2(90., 90.), &[target]).is_none());
        assert!(resolve_drop::<Panel>(&Panel::Inspector, egui::pos2(f32::NAN, 0.), &[]).is_none());
        assert_eq!(a.docking.layout, layout);
        assert_eq!(retained(&a), before);
    }
    #[test]
    fn tabbing_away_canvas_clears_hidden_control_audit_and_retains_native_content() {
        let (_area, mut a, ctx) = app();
        render(&mut a, &ctx);
        assert!(a.views.toolbar_bounds().count() > 0);
        let before = retained(&a);
        a.apply_dock(
            a.docking.revision,
            vec![
                Action::Move {
                    panel: Panel::Canvas,
                    anchor: Panel::Inspector,
                    placement: Placement::Tab { before: None },
                },
                Action::Activate {
                    panel: Panel::Inspector,
                },
            ],
        )
        .unwrap();
        render(&mut a, &ctx);
        assert_eq!(a.views.toolbar_bounds().count(), 0);
        assert_eq!(retained(&a), before);
        assert_eq!(a.docking.layout.panels().len(), 3);
    }
}

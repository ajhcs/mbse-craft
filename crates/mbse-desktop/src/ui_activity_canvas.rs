//! Render the active activity diagram using current semantic endpoints and guarded layout.
use super::activity_editor::{Drag, Rendered, guard_text};
use super::*;
use mbse_model::{
    native_views::{self as nv, OccurrenceId, Stamp, ViewId, activities as va},
    types::{self as ty, Id, activities as ac, activities::forms::Form},
};
impl Workspace {
    pub(super) fn activity_select_element(
        &mut self,
        stamp: &Stamp,
        view: ViewId,
        id: Id,
    ) -> Result<(), String> {
        if self.views.active != view || *stamp != self.views.views.stamp(&self.session.editor) {
            return Err("Activity diagram source/view changed; selection retained.".into());
        }
        let d = self
            .views
            .views
            .document()
            .activity_diagrams
            .get(&view)
            .ok_or("No active activity diagram")?;
        if d.source_document != self.session.editor.snapshot.document_id {
            return Err("Foreign diagram source; no ID is rebound.".into());
        }
        let store = ty::load(&self.session.editor.snapshot)?;
        let a = store
            .activities
            .get(&d.activity)
            .ok_or("Activity missing; original diagram retained")?;
        let form = if id == a.id {
            Form::Activity {
                id: Some(a.id),
                system: a.system.0.to_string(),
                name: a.name.clone(),
            }
        } else if let Some(n) = a.nodes.get(&id) {
            Form::Node {
                activity: a.id,
                id: Some(id),
                name: n.name.clone(),
                node_kind: n.kind,
                partition: n.partition.map_or(String::new(), |id| id.0.to_string()),
            }
        } else if let Some(p) = a.partitions.get(&id) {
            Form::Partition {
                activity: a.id,
                id: Some(id),
                name: p.name.clone(),
            }
        } else if let Some(p) = a.pins.get(&id) {
            Form::Pin {
                activity: a.id,
                id: Some(id),
                owner: p.owner.0.to_string(),
                name: p.name.clone(),
                direction: p.direction,
                value_type: p.value_type.0.to_string(),
            }
        } else if let Some(f) = a.flows.get(&id) {
            Form::Flow {
                form: ac::FlowForm {
                    activity: a.id,
                    id: Some(id),
                    name: f.name.clone(),
                    kind: f.kind,
                    source: f.source.0.to_string(),
                    target: f.target.0.to_string(),
                    guard: guard_text(f.guard),
                },
            }
        } else {
            return Err("Exact activity element is missing; no replacement is selected.".into());
        };
        self.activity_pick(form, Some(stamp))
    }
    pub(super) fn activity_canvas(&mut self, ui: &mut egui::Ui) -> bool {
        let view = self.views.active;
        let Some(diagram) = self
            .views
            .views
            .document()
            .activity_diagrams
            .get(&view)
            .cloned()
        else {
            return false;
        };
        let stamp = self.views.views.stamp(&self.session.editor);
        let camera = self.views.views.document().views[&view].camera.clone();
        let store = match ty::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(error) => {
                ui.colored_label(ERROR, error);
                return true;
            }
        };
        let projection = match va::project(&diagram, &self.session.editor.snapshot) {
            Ok(p) => p,
            Err(error) => {
                ui.colored_label(ERROR, error);
                return true;
            }
        };
        if self.activities.drag.as_ref().is_some_and(|d| {
            d.view != view || d.stamp != stamp || d.viewport != self.context.content_rect()
        }) {
            self.activities.drag = None;
            self.activities.note =
                "Diagram/source changed during gesture; original geometry retained.".into();
        }
        if self.context.input(|i| i.key_pressed(egui::Key::Escape))
            && self.activities.drag.is_some()
        {
            self.activities.drag = None;
            self.activities.note =
                "Unpublished gesture cancelled; original geometry retained.".into();
        }
        let ready = !self.dock_blocked_without_activity() && !self.activity_pending();
        let mut history = None;
        let mut change = None;
        let mut select = None;
        ui.heading(format!("Activity diagram · view #{}", view.get()));
        if let Some(a) = store.activities.get(&diagram.activity) {
            ui.label(format!(
                "Activity #{} · {} · System #{}",
                a.id.0, a.name, a.system.0
            ));
        } else {
            ui.colored_label(
                ERROR,
                format!(
                    "Activity #{} missing; original presentation is retained.",
                    diagram.activity.0
                ),
            );
        }
        for limit in &projection.limitations {
            ui.label(limit);
        }
        ui.label("Native card viewpoint filters do not filter activity elements. Edit activity data through its dedicated editor.");
        ui.horizontal_wrapped(|ui| {
            let r = ui.add_enabled(ready, egui::Button::new("Open activity editor"));
            self.bound("activity-canvas:Open editor", &r);
            if r.clicked() {
                select = Some(diagram.activity);
            }
            for (redo, label) in [(false, "Diagram Undo"), (true, "Diagram Redo")] {
                let r = ui.add_enabled(
                    ready && self.views.views.can_layout_history(redo),
                    egui::Button::new(label),
                );
                self.bound(format!("activity-canvas:{label}"), &r);
                if r.clicked() {
                    history = Some(redo);
                }
            }
            let r = ui.add_enabled(
                self.activities.drag.is_some(),
                egui::Button::new("Cancel gesture"),
            );
            self.bound("activity-canvas:Cancel gesture", &r);
            if r.clicked() {
                self.activities.drag = None;
            }
        });
        let selected = self
            .activities
            .selected
            .filter(|(owner, id)| *owner == view && diagram.occurrences.contains_key(id))
            .map(|(_, id)| diagram.occurrences[&id].clone());
        let caption = selected.as_ref().map_or(
            "Select an occurrence to change its presentation.".into(),
            |o| {
                format!(
                    "Occurrence {} → {} #{}",
                    o.id.get(),
                    kind_label(o.expected),
                    o.element.0
                )
            },
        );
        ui.add_sized(
            [ui.available_width(), 36.],
            egui::Label::new(caption).wrap(),
        );
        ui.horizontal_wrapped(|ui| {
            for (label, dx, dy) in [
                ("Move left", -40., 0.),
                ("Move right", 40., 0.),
                ("Move up", 0., -40.),
                ("Move down", 0., 40.),
            ] {
                let r = ui.add_enabled(
                    ready && selected.as_ref().is_some_and(|o| !o.pinned),
                    egui::Button::new(label),
                );
                self.bound(format!("activity-canvas:{label}"), &r);
                if r.clicked() {
                    let o = selected.as_ref().unwrap();
                    let mut geometry = o.geometry.clone();
                    geometry.x += dx;
                    geometry.y += dy;
                    change = Some(va::Layout::Move {
                        view,
                        occurrence: o.id,
                        geometry,
                    });
                }
            }
            let r = ui.add_enabled(
                ready && selected.is_some(),
                egui::Button::new(if selected.as_ref().is_none_or(|o| o.visible) {
                    "Hide occurrence"
                } else {
                    "Show occurrence"
                }),
            );
            self.bound("activity-canvas:Toggle visibility", &r);
            if r.clicked() {
                let o = selected.as_ref().unwrap();
                change = Some(va::Layout::Visible {
                    view,
                    occurrence: o.id,
                    visible: !o.visible,
                });
            }
            let r = ui.add_enabled(
                ready && selected.is_some(),
                egui::Button::new(if selected.as_ref().is_some_and(|o| o.pinned) {
                    "Unpin occurrence"
                } else {
                    "Pin occurrence"
                }),
            );
            self.bound("activity-canvas:Toggle pin", &r);
            if r.clicked() {
                let o = selected.as_ref().unwrap();
                change = Some(va::Layout::Pin {
                    view,
                    occurrence: o.id,
                    pinned: !o.pinned,
                });
            }
            let r = ui.add_enabled(
                ready && selected.is_some(),
                egui::Button::new("Remove occurrence only"),
            );
            self.bound("activity-canvas:Remove occurrence", &r);
            if r.clicked() {
                change = Some(va::Layout::Remove {
                    view,
                    occurrence: selected.as_ref().unwrap().id,
                });
            }
        });
        ui.label(&self.activities.note);
        let source_activity = projection
            .activity_resolved
            .then(|| store.activities.get(&diagram.activity))
            .flatten();
        let mut occurrences: Vec<_> = diagram
            .occurrences
            .values()
            .filter(|o| {
                o.visible
                    && o.presence.active()
                    && source_activity.and_then(|a| va::kind(a, o.element)) == Some(o.expected)
            })
            .cloned()
            .collect();
        let mut preview = false;
        for o in &mut occurrences {
            if let Some(d) = self
                .activities
                .drag
                .as_ref()
                .filter(|d| d.view == view && d.occurrence == o.id)
            {
                let pointer = self
                    .context
                    .input(|i| i.pointer.latest_pos())
                    .unwrap_or(d.start);
                o.geometry = d.geometry.clone();
                o.geometry.x += (pointer.x - d.start.x) as f64 / camera.zoom;
                o.geometry.y += (pointer.y - d.start.y) as f64 / camera.zoom;
                preview = true;
            }
        }
        let scale = camera.zoom as f32;
        let mut rendered_flows = Vec::new();
        let height = (ui.available_height() * 0.66).max(320.);
        egui::ScrollArea::both()
            .id_salt(("activity-diagram", view.get()))
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = occurrences
                    .iter()
                    .map(|o| (o.geometry.x - camera.x + o.geometry.width + 60.) as f32 * scale)
                    .fold(ui.available_width().max(400.), f32::max)
                    .clamp(400., 32768.);
                let height = occurrences
                    .iter()
                    .map(|o| (o.geometry.y - camera.y + o.geometry.height + 60.) as f32 * scale)
                    .fold(400., f32::max)
                    .clamp(400., 32768.);
                let (area, _) =
                    ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
                if self
                    .activities
                    .drag
                    .as_ref()
                    .is_some_and(|d| d.canvas_origin != area.min)
                {
                    self.activities.drag = None;
                    preview = false;
                    for o in &mut occurrences {
                        o.geometry = diagram.occurrences[&o.id].geometry.clone();
                    }
                    self.activities.note =
                        "Canvas moved during gesture; original geometry retained.".into();
                }
                let rects: BTreeMap<_, _> = occurrences
                    .iter()
                    .map(|o| {
                        (
                            o.id,
                            Rect::from_min_size(
                                area.min
                                    + egui::vec2(
                                        (o.geometry.x - camera.x) as f32,
                                        (o.geometry.y - camera.y) as f32,
                                    ) * scale,
                                egui::vec2(o.geometry.width as f32, o.geometry.height as f32)
                                    * scale,
                            ),
                        )
                    })
                    .collect();
                for flow in &projection.routes {
                    if let (Some(a), Some(b)) = (rects.get(&flow.source), rects.get(&flow.target)) {
                        let color = if flow.kind == ac::FlowKind::Control {
                            ACCENT
                        } else {
                            Color32::from_rgb(36, 115, 86)
                        };
                        let start = a.center_bottom();
                        let end = b.center_top();
                        if flow.source == flow.target {
                            let loop_rect =
                                Rect::from_min_size(a.right_top(), egui::vec2(55., a.height()));
                            ui.painter().rect_stroke(
                                loop_rect,
                                8.,
                                Stroke::new(1.5, color),
                                egui::StrokeKind::Middle,
                            );
                        } else {
                            ui.painter()
                                .arrow(start, end - start, Stroke::new(1.5, color));
                        }
                        let guard = guard_text(flow.guard);
                        if !guard.is_empty() {
                            ui.painter().text(
                                start.lerp(end, 0.5) + egui::vec2(8., 0.),
                                egui::Align2::LEFT_CENTER,
                                format!("[{guard}]"),
                                FontId::proportional(12.),
                                color,
                            );
                        }
                        rendered_flows.push(flow.flow);
                    }
                }
                for o in &occurrences {
                    let rect = rects[&o.id];
                    let selected = self.activities.selected == Some((view, o.id));
                    let stroke = Stroke::new(
                        if selected { 2.5 } else { 1. },
                        if selected { ACCENT } else { LINE },
                    );
                    match o.expected {
                        va::Kind::Node(ac::NodeKind::Decision | ac::NodeKind::Merge) => {
                            ui.painter().add(egui::Shape::convex_polygon(
                                vec![
                                    rect.center_top(),
                                    rect.right_center(),
                                    rect.center_bottom(),
                                    rect.left_center(),
                                ],
                                PAPER,
                                stroke,
                            ));
                        }
                        va::Kind::Node(ac::NodeKind::Initial | ac::NodeKind::Final) => {
                            let center = rect.center_top() + egui::vec2(0., 30. * scale);
                            ui.painter().circle_filled(center, 13. * scale, INK);
                            if o.expected == va::Kind::Node(ac::NodeKind::Final) {
                                ui.painter().circle_stroke(
                                    center,
                                    19. * scale,
                                    Stroke::new(2., ACCENT),
                                );
                            }
                        }
                        _ => {
                            ui.painter().rect(
                                rect,
                                if matches!(o.expected, va::Kind::Partition) {
                                    0.
                                } else {
                                    7.
                                },
                                PAPER,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                    let label = source_activity
                        .map(|a| element_name(a, o.element))
                        .unwrap_or_default();
                    let text = format!("{} #{}\n{}", kind_label(o.expected), o.element.0, label);
                    let galley = ui.painter().layout(
                        text,
                        FontId::proportional(13. * scale),
                        INK,
                        (rect.width() - 28. * scale).max(1.),
                    );
                    let top = if matches!(
                        o.expected,
                        va::Kind::Node(ac::NodeKind::Initial | ac::NodeKind::Final)
                    ) {
                        58. * scale
                    } else {
                        14. * scale
                    };
                    ui.painter().with_clip_rect(rect).galley(
                        rect.min + egui::vec2(14. * scale, top),
                        galley,
                        INK,
                    );
                    let r = ui
                        .interact(
                            rect,
                            ui.id().with(("activity-occurrence", o.id.get())),
                            egui::Sense::click_and_drag(),
                        )
                        .on_hover_text(format!(
                            "Activity #{} · element #{} · {label}",
                            diagram.activity.0, o.element.0
                        ));
                    self.bound(format!("activity-occurrence:{}", o.id.get()), &r);
                    if r.drag_started() && ready && !o.pinned {
                        if let Some(pointer) = r
                            .ctx
                            .input(|i| i.pointer.press_origin())
                            .or_else(|| r.interact_pointer_pos())
                        {
                            self.activities.drag = Some(Drag {
                                view,
                                occurrence: o.id,
                                stamp: stamp.clone(),
                                geometry: o.geometry.clone(),
                                start: pointer,
                                viewport: self.context.content_rect(),
                                canvas_origin: area.min,
                            });
                            self.activities.selected = Some((view, o.id));
                        }
                    }
                    if r.drag_stopped() {
                        if let Some(d) = self.activities.drag.take() {
                            if d.view == view && d.occurrence == o.id {
                                let pointer =
                                    r.ctx.input(|i| i.pointer.latest_pos()).unwrap_or(d.start);
                                let mut geometry = d.geometry;
                                geometry.x += (pointer.x - d.start.x) as f64 / camera.zoom;
                                geometry.y += (pointer.y - d.start.y) as f64 / camera.zoom;
                                let result = self.activity_layout(
                                    &d.stamp,
                                    va::Layout::Move {
                                        view,
                                        occurrence: o.id,
                                        geometry,
                                    },
                                );
                                self.activities.note = result
                                    .map(|_| {
                                        "Diagram moved once; native activity data unchanged.".into()
                                    })
                                    .unwrap_or_else(|e| e);
                            }
                        }
                    }
                    if r.clicked() && ready {
                        self.activities.selected = Some((view, o.id));
                        select = Some(o.element);
                    }
                }
            });
        self.activities.rendered = Some(Rendered {
            view,
            stamp: stamp.clone(),
            occurrences: occurrences
                .iter()
                .map(|o| (o.id, o.element, o.geometry.clone()))
                .collect(),
            flows: rendered_flows,
            preview,
        });
        ui.separator();
        ui.label(format!(
            "Activity flow ledger · {} shown · {} without unique visible endpoints",
            projection.routes.len(),
            projection.unshown_flows
        ));
        if let Some(a) = source_activity {
            for flow in a.flows.values() {
                let r = ui.button(format!(
                    "{:?} #{} · {} · {} → {} · guard {}",
                    flow.kind,
                    flow.id.0,
                    flow.name,
                    flow.source.0,
                    flow.target.0,
                    guard_text(flow.guard)
                ));
                self.bound(format!("activity-flow:{}", flow.id.0), &r);
                if r.clicked() && ready {
                    select = Some(flow.id);
                }
            }
        }
        if let Some(redo) = history {
            let result = if self.dock_blocked_without_activity() || self.activity_pending() {
                Err("Finish pending work before diagram history.".into())
            } else {
                self.views
                    .views
                    .undo(&self.session.editor, &stamp, redo)
                    .map(|_| ())
            };
            self.views.reconcile();
            self.activities.note = result
                .map(|_| "Diagram history restored; native data unchanged.".into())
                .unwrap_or_else(|e| e);
        } else if let Some(action) = change {
            self.activities.note = self
                .activity_layout(&stamp, action)
                .map(|_| "Diagram presentation updated; native activity data unchanged.".into())
                .unwrap_or_else(|e| e);
        } else if let Some(id) = select {
            if let Err(error) = self.activity_select_element(&stamp, view, id) {
                self.activities.note = error;
            }
        }
        true
    }
}
fn element_name(a: &ac::Activity, id: Id) -> String {
    if let Some(n) = a.nodes.get(&id) {
        n.name.clone()
    } else if let Some(p) = a.partitions.get(&id) {
        p.name.clone()
    } else {
        a.pins.get(&id).map_or(String::new(), |p| {
            format!(
                "{} · Action #{} · Type #{}",
                p.name, p.owner.0, p.value_type.0
            )
        })
    }
}

fn kind_label(kind: va::Kind) -> &'static str {
    match kind {
        va::Kind::Partition => "Partition",
        va::Kind::Node(ac::NodeKind::Initial) => "Initial",
        va::Kind::Node(ac::NodeKind::Action) => "Action",
        va::Kind::Node(ac::NodeKind::Decision) => "Decision",
        va::Kind::Node(ac::NodeKind::Merge) => "Merge",
        va::Kind::Node(ac::NodeKind::Final) => "Final",
        va::Kind::Pin(ac::Direction::In) => "Input pin",
        va::Kind::Pin(ac::Direction::Out) => "Output pin",
    }
}

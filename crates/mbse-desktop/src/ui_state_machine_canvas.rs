//! Render the active state-machine diagram using current semantic endpoints and guarded layout.
use super::activity_canvas::{circle_glyph, flow_endpoints, paint_flow_arrow, symbol_boundary};
use super::activity_editor::{Drag, guard_text};
use super::state_machine_editor::Rendered;
use super::*;
use mbse_model::{
    native_views::{self as nv, Stamp, ViewId, state_machines as va},
    types::{self as ty, Id, state_machines as sm, state_machines::Form},
};
impl Workspace {
    pub(super) fn sm_select_element(
        &mut self,
        stamp: &Stamp,
        view: ViewId,
        id: Id,
    ) -> Result<(), String> {
        if self.views.active != view || *stamp != self.views.views.stamp(&self.session.editor) {
            return Err("State-machine diagram source/view changed; selection retained.".into());
        }
        let d = self
            .views
            .views
            .document()
            .state_machine_diagrams
            .get(&view)
            .ok_or("No active state-machine diagram")?;
        if d.source_document != self.session.editor.snapshot.document_id {
            return Err("Foreign state-machine diagram source; no ID is rebound.".into());
        }
        let store = ty::load(&self.session.editor.snapshot)?;
        let a = store
            .state_machines
            .get(&d.machine)
            .ok_or("State machine missing; original diagram retained")?;
        let form = if id == a.id {
            Form::Machine {
                id: Some(a.id),
                system: a.system.0.to_string(),
                name: a.name.clone(),
            }
        } else if let Some(n) = a.states.get(&id) {
            Form::State {
                machine: a.id,
                id: Some(id),
                name: n.name.clone(),
                state_kind: n.kind,
            }
        } else if let Some(e) = a.events.get(&id) {
            Form::Event {
                machine: a.id,
                id: Some(id),
                name: e.name.clone(),
            }
        } else if let Some(t) = a.transitions.get(&id) {
            Form::Transition {
                machine: a.id,
                id: Some(id),
                name: t.name.clone(),
                source: t.source.0.to_string(),
                target: t.target.0.to_string(),
                event: t.event.map_or(String::new(), |id| id.0.to_string()),
                guard: guard_text(t.guard),
            }
        } else {
            return Err(
                "Exact state-machine element is missing; no replacement is selected.".into(),
            );
        };
        self.sm_pick(form, Some(stamp))
    }
    pub(super) fn sm_canvas(&mut self, ui: &mut egui::Ui) -> bool {
        let view = self.views.active;
        let Some(diagram) = self
            .views
            .views
            .document()
            .state_machine_diagrams
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
        if self.state_machines.drag.as_ref().is_some_and(|d| {
            d.view != view || d.stamp != stamp || d.viewport != self.context.content_rect()
        }) {
            self.state_machines.drag = None;
            self.state_machines.note =
                "Diagram/source changed during gesture; original geometry retained.".into();
        }
        if self.context.input(|i| i.key_pressed(egui::Key::Escape))
            && self.state_machines.drag.is_some()
        {
            self.state_machines.drag = None;
            self.state_machines.note =
                "Unpublished gesture cancelled; original geometry retained.".into();
        }
        let ready = !self.dock_blocked_without_state_machine() && !self.sm_pending();
        let mut history = None;
        let mut change = None;
        let mut select = None;
        ui.heading(format!("State-machine diagram · view #{}", view.get()));
        if let Some(a) = store.state_machines.get(&diagram.machine) {
            ui.label(format!(
                "Machine #{} · {} · System #{}",
                a.id.0, a.name, a.system.0
            ));
        } else {
            ui.colored_label(
                ERROR,
                format!(
                    "Machine #{} missing; original presentation is retained.",
                    diagram.machine.0
                ),
            );
        }
        for limit in &projection.limitations {
            ui.label(limit);
        }
        ui.label("Native card viewpoint filters do not filter state-machine states. Edit state-machine data through its dedicated editor.");
        ui.horizontal_wrapped(|ui| {
            let r = ui.add_enabled(ready, egui::Button::new("Open state-machine editor"));
            self.bound("state-machine-canvas:Open editor", &r);
            if r.clicked() {
                select = Some(diagram.machine);
            }
            for (redo, label) in [(false, "Diagram Undo"), (true, "Diagram Redo")] {
                let r = ui.add_enabled(
                    ready && self.views.views.can_layout_history(redo),
                    egui::Button::new(label),
                );
                self.bound(format!("state-machine-canvas:{label}"), &r);
                if r.clicked() {
                    history = Some(redo);
                }
            }
            let r = ui.add_enabled(
                self.state_machines.drag.is_some(),
                egui::Button::new("Cancel gesture"),
            );
            self.bound("state-machine-canvas:Cancel gesture", &r);
            if r.clicked() {
                self.state_machines.drag = None;
            }
        });
        let selected = self
            .state_machines
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
                self.bound(format!("state-machine-canvas:{label}"), &r);
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
            self.bound("state-machine-canvas:Toggle visibility", &r);
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
            self.bound("state-machine-canvas:Toggle pin", &r);
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
            self.bound("state-machine-canvas:Remove occurrence", &r);
            if r.clicked() {
                change = Some(va::Layout::Remove {
                    view,
                    occurrence: selected.as_ref().unwrap().id,
                });
            }
        });
        ui.label(&self.state_machines.note);
        let source_machine = projection
            .machine_resolved
            .then(|| store.state_machines.get(&diagram.machine))
            .flatten();
        let mut occurrences: Vec<_> = diagram
            .occurrences
            .values()
            .filter(|o| {
                o.visible
                    && o.presence.active()
                    && source_machine.and_then(|a| va::kind(a, o.element)) == Some(o.expected)
            })
            .cloned()
            .collect();
        let mut preview = false;
        for o in &mut occurrences {
            if let Some(d) = self
                .state_machines
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
        let mut rendered_transitions = Vec::new();
        let mut endpoints = Vec::new();
        let mut overlapping = 0usize;
        let height = (ui.available_height() * 0.66).max(320.);
        egui::ScrollArea::both()
            .id_salt(("state-machine-diagram", view.get()))
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
                    .state_machines
                    .drag
                    .as_ref()
                    .is_some_and(|d| d.canvas_origin != area.min)
                {
                    self.state_machines.drag = None;
                    preview = false;
                    for o in &mut occurrences {
                        o.geometry = diagram.occurrences[&o.id].geometry.clone();
                    }
                    self.state_machines.note =
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
                        let color = ACCENT;
                        let source_kind = glyph(diagram.occurrences[&flow.source].expected);
                        let target_kind = glyph(diagram.occurrences[&flow.target].expected);
                        let (start, end) = if flow.source == flow.target {
                            let start = symbol_boundary(*a, source_kind, scale, egui::Vec2::X);
                            let end = symbol_boundary(*a, target_kind, scale, -egui::Vec2::Y);
                            let upper = egui::pos2(a.right() + 50., a.top() - 35.);
                            ui.painter().line_segment(
                                [start, egui::pos2(upper.x, start.y)],
                                Stroke::new(1.5, color),
                            );
                            ui.painter().line_segment(
                                [egui::pos2(upper.x, start.y), upper],
                                Stroke::new(1.5, color),
                            );
                            let last = egui::pos2(end.x, upper.y);
                            ui.painter()
                                .line_segment([upper, last], Stroke::new(1.5, color));
                            paint_flow_arrow(ui.painter(), last, end, Stroke::new(1.5, color));
                            (start, end)
                        } else {
                            let Some((start, end)) =
                                flow_endpoints(*a, source_kind, *b, target_kind, scale)
                            else {
                                overlapping += 1;
                                continue;
                            };
                            paint_flow_arrow(ui.painter(), start, end, Stroke::new(1.5, color));
                            (start, end)
                        };
                        endpoints.push((flow.transition, [start.x, start.y], [end.x, end.y]));
                        let guard = guard_text(flow.guard);
                        let event = flow.event.map_or("completion".into(), |id| {
                            source_machine
                                .and_then(|m| m.events.get(&id))
                                .map_or(format!("Event #{}", id.0), |e| {
                                    format!("{} #{}", e.name, id.0)
                                })
                        });
                        if !guard.is_empty() || flow.event.is_some() {
                            ui.painter().text(
                                start.lerp(end, 0.5) + egui::vec2(8., 0.),
                                egui::Align2::LEFT_CENTER,
                                format!("{event} [{guard}]"),
                                FontId::proportional(12.),
                                color,
                            );
                        }
                        rendered_transitions.push(flow.transition);
                    }
                }
                for o in &occurrences {
                    let rect = rects[&o.id];
                    let selected = self.state_machines.selected == Some((view, o.id));
                    let stroke = Stroke::new(
                        if selected { 2.5 } else { 1. },
                        if selected { ACCENT } else { LINE },
                    );
                    match o.expected {
                        sm::StateKind::Initial | sm::StateKind::Final => {
                            let (center, radius) = circle_glyph(rect, glyph(o.expected), scale);
                            ui.painter().circle_filled(
                                center,
                                if o.expected == sm::StateKind::Final {
                                    (radius - 6. * scale).max(1.)
                                } else {
                                    radius
                                },
                                INK,
                            );
                            if o.expected == sm::StateKind::Final {
                                ui.painter()
                                    .circle_stroke(center, radius, Stroke::new(2., ACCENT));
                            }
                        }
                        sm::StateKind::State => {
                            ui.painter()
                                .rect(rect, 7., PAPER, stroke, egui::StrokeKind::Inside);
                        }
                    }
                    let label = source_machine
                        .map(|a| element_name(a, o.element))
                        .unwrap_or_default();
                    let text = format!("{} #{}\n{}", kind_label(o.expected), o.element.0, label);
                    let galley = ui.painter().layout(
                        text,
                        FontId::proportional(13. * scale),
                        INK,
                        (rect.width() - 28. * scale).max(1.),
                    );
                    let top = if matches!(o.expected, sm::StateKind::Initial | sm::StateKind::Final)
                    {
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
                            ui.id().with(("state-machine-occurrence", o.id.get())),
                            egui::Sense::click_and_drag(),
                        )
                        .on_hover_text(format!(
                            "Machine #{} · element #{} · {label}",
                            diagram.machine.0, o.element.0
                        ));
                    self.bound(format!("state-machine-occurrence:{}", o.id.get()), &r);
                    if r.drag_started() && ready && !o.pinned {
                        if let Some(pointer) = r
                            .ctx
                            .input(|i| i.pointer.press_origin())
                            .or_else(|| r.interact_pointer_pos())
                        {
                            self.state_machines.drag = Some(Drag {
                                view,
                                occurrence: o.id,
                                stamp: stamp.clone(),
                                geometry: o.geometry.clone(),
                                start: pointer,
                                viewport: self.context.content_rect(),
                                canvas_origin: area.min,
                            });
                            self.state_machines.selected = Some((view, o.id));
                        }
                    }
                    if r.drag_stopped() {
                        if let Some(d) = self.state_machines.drag.take() {
                            if d.view == view && d.occurrence == o.id {
                                let pointer =
                                    r.ctx.input(|i| i.pointer.latest_pos()).unwrap_or(d.start);
                                let mut geometry = d.geometry;
                                geometry.x += (pointer.x - d.start.x) as f64 / camera.zoom;
                                geometry.y += (pointer.y - d.start.y) as f64 / camera.zoom;
                                let result = self.sm_layout(
                                    &d.stamp,
                                    va::Layout::Move {
                                        view,
                                        occurrence: o.id,
                                        geometry,
                                    },
                                );
                                self.state_machines.note = result
                                    .map(|_| {
                                        "Diagram moved once; native state-machine data unchanged."
                                            .into()
                                    })
                                    .unwrap_or_else(|e| e);
                            }
                        }
                    }
                    if r.clicked() && ready {
                        self.state_machines.selected = Some((view, o.id));
                        select = Some(o.element);
                    }
                }
            });
        self.state_machines.rendered = Some(Rendered {
            view,
            stamp: stamp.clone(),
            occurrences: occurrences
                .iter()
                .map(|o| (o.id, o.element, o.geometry.clone()))
                .collect(),
            transitions: rendered_transitions.clone(),
            endpoints,
            overlapping,
            preview,
        });
        ui.separator();
        ui.label(format!(
            "State-machine transition ledger · {} arrows shown · {} without unique visible endpoints · {} overlapping state pairs (move them apart)",
            rendered_transitions.len(),
            projection.unshown_transitions,overlapping
        ));
        if let Some(a) = source_machine {
            for transition in a.transitions.values() {
                let event = transition.event.map_or("completion".into(), |id| {
                    a.events.get(&id).map_or(format!("Event #{}", id.0), |e| {
                        format!("Event #{} · {}", id.0, e.name)
                    })
                });
                let r = ui.button(format!(
                    "Transition #{} · {} · {} → {} · {} · guard {}",
                    transition.id.0,
                    transition.name,
                    transition.source.0,
                    transition.target.0,
                    event,
                    guard_text(transition.guard)
                ));
                self.bound(format!("state-machine-transition:{}", transition.id.0), &r);
                if r.clicked() && ready {
                    select = Some(transition.id);
                }
            }
        }
        if let Some(redo) = history {
            let result = if self.dock_blocked_without_state_machine() || self.sm_pending() {
                Err("Finish pending work before diagram history.".into())
            } else {
                self.views
                    .views
                    .undo(&self.session.editor, &stamp, redo)
                    .map(|_| ())
            };
            self.views.reconcile();
            self.state_machines.note = result
                .map(|_| "Diagram history restored; native data unchanged.".into())
                .unwrap_or_else(|e| e);
        } else if let Some(action) = change {
            self.state_machines.note = self
                .sm_layout(&stamp, action)
                .map(|_| {
                    "Diagram presentation updated; native state-machine data unchanged.".into()
                })
                .unwrap_or_else(|e| e);
        } else if let Some(id) = select {
            if let Err(error) = self.sm_select_element(&stamp, view, id) {
                self.state_machines.note = error;
            }
        }
        true
    }
}
fn glyph(kind: sm::StateKind) -> nv::activities::Kind {
    use mbse_model::types::activities::NodeKind;
    nv::activities::Kind::Node(match kind {
        sm::StateKind::Initial => NodeKind::Initial,
        sm::StateKind::State => NodeKind::Action,
        sm::StateKind::Final => NodeKind::Final,
    })
}
fn element_name(a: &sm::Machine, id: Id) -> String {
    a.states.get(&id).map_or(String::new(), |n| n.name.clone())
}
fn kind_label(kind: sm::StateKind) -> &'static str {
    match kind {
        sm::StateKind::Initial => "Initial",
        sm::StateKind::State => "State",
        sm::StateKind::Final => "Final",
    }
}

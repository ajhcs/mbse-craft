//! Render the active scenario diagram using current semantic endpoints and guarded layout.
use super::activity_canvas::paint_flow_arrow;
use super::activity_editor::Drag;
use super::scenario_editor::Rendered;
use super::*;
use mbse_model::{
    native_views::{self as nv, Stamp, ViewId, scenarios as va},
    types::{
        self as ty, Id,
        scenarios::{Form, MessageKind},
    },
};
impl Workspace {
    pub(super) fn sc_select_element(
        &mut self,
        stamp: &Stamp,
        view: ViewId,
        id: Id,
    ) -> Result<(), String> {
        if self.views.active != view || *stamp != self.views.views.stamp(&self.session.editor) {
            return Err("Sequence diagram source/view changed; selection retained.".into());
        }
        let d = self
            .views
            .views
            .document()
            .scenario_diagrams
            .get(&view)
            .ok_or("No active scenario diagram")?;
        if d.source_document != self.session.editor.snapshot.document_id {
            return Err("Foreign scenario diagram source; no ID is rebound.".into());
        }
        let store = ty::load(&self.session.editor.snapshot)?;
        let a = store
            .scenarios
            .get(&d.scenario)
            .ok_or("State scenario missing; original diagram retained")?;
        let form = super::scenario_editor::accepted_form(a, id)
            .ok_or("Exact scenario element missing; no identity is rebound.")?;
        self.sc_pick(form, Some(stamp))
    }
    pub(super) fn sc_canvas(&mut self, ui: &mut egui::Ui) -> bool {
        let view = self.views.active;
        let Some(diagram) = self
            .views
            .views
            .document()
            .scenario_diagrams
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
        if self.scenarios.drag.as_ref().is_some_and(|d| {
            d.view != view || d.stamp != stamp || d.viewport != self.context.content_rect()
        }) {
            self.scenarios.drag = None;
            self.scenarios.note =
                "Diagram/source changed during gesture; original geometry retained.".into();
        }
        if self.context.input(|i| i.key_pressed(egui::Key::Escape)) && self.scenarios.drag.is_some()
        {
            self.scenarios.drag = None;
            self.scenarios.note =
                "Unpublished gesture cancelled; original geometry retained.".into();
        }
        let ready = !self.dock_blocked_without_scenario() && !self.sc_pending();
        let mut history = None;
        let mut change = None;
        let mut select = None;
        ui.heading(format!("Sequence diagram · view #{}", view.get()));
        if let Some(a) = store.scenarios.get(&diagram.scenario) {
            ui.label(format!(
                "Scenario #{} · {} · System #{}",
                a.id.0, a.name, a.system.0
            ));
        } else {
            ui.colored_label(
                ERROR,
                format!(
                    "Scenario #{} missing; original presentation is retained.",
                    diagram.scenario.0
                ),
            );
        }
        for limit in &projection.limitations {
            ui.label(limit);
        }
        ui.label("Lifeline headers bind exact participants. Filled arrows: synchronous; open: asynchronous; dashed: reply. Guard text and order do not execute.");
        ui.horizontal_wrapped(|ui| {
            let r = ui.add_enabled(ready, egui::Button::new("Open scenario editor"));
            self.bound("scenario-canvas:Open editor", &r);
            if r.clicked() {
                select = Some(diagram.scenario);
            }
            for (redo, label) in [(false, "Diagram Undo"), (true, "Diagram Redo")] {
                let r = ui.add_enabled(
                    ready && self.views.views.can_layout_history(redo),
                    egui::Button::new(label),
                );
                self.bound(format!("scenario-canvas:{label}"), &r);
                if r.clicked() {
                    history = Some(redo);
                }
            }
            let r = ui.add_enabled(
                self.scenarios.drag.is_some(),
                egui::Button::new("Cancel gesture"),
            );
            self.bound("scenario-canvas:Cancel gesture", &r);
            if r.clicked() {
                self.scenarios.drag = None;
            }
        });
        let selected = self
            .scenarios
            .selected
            .filter(|(owner, id)| *owner == view && diagram.occurrences.contains_key(id))
            .map(|(_, id)| diagram.occurrences[&id].clone());
        let caption = selected.as_ref().map_or(
            "Select an occurrence to change its presentation.".into(),
            |o| {
                format!(
                    "Occurrence {} → {} #{}",
                    o.id.get(),
                    "Lifeline",
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
                self.bound(format!("scenario-canvas:{label}"), &r);
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
            self.bound("scenario-canvas:Toggle visibility", &r);
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
            self.bound("scenario-canvas:Toggle pin", &r);
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
            self.bound("scenario-canvas:Remove occurrence", &r);
            if r.clicked() {
                change = Some(va::Layout::Remove {
                    view,
                    occurrence: selected.as_ref().unwrap().id,
                });
            }
        });
        ui.label(&self.scenarios.note);
        let source_scenario = projection
            .scenario_resolved
            .then(|| store.scenarios.get(&diagram.scenario))
            .flatten();
        let mut occurrences: Vec<_> = diagram
            .occurrences
            .values()
            .filter(|o| {
                o.visible
                    && o.presence.active()
                    && source_scenario.and_then(|a| va::kind(a, o.element)) == Some(o.expected)
            })
            .cloned()
            .collect();
        let mut preview = false;
        for o in &mut occurrences {
            if let Some(d) = self
                .scenarios
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
        let mut rendered_messages = Vec::new();
        let mut endpoints = Vec::new();
        let mut labels = Vec::new();
        let height = (ui.available_height() * 0.66).max(320.);
        egui::ScrollArea::both()
            .id_salt(("scenario-diagram", view.get()))
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = occurrences
                    .iter()
                    .map(|o| (o.geometry.x - camera.x + o.geometry.width + 60.) as f32 * scale)
                    .fold(ui.available_width().max(400.), f32::max)
                    .clamp(400., 32768.);
                let base_y = occurrences
                    .iter()
                    .map(|o| o.geometry.y + o.geometry.height)
                    .fold(0., f64::max)
                    + 48.;
                let world_bottom = base_y + (projection.rows.len() + 1) as f64 * va::ROW_HEIGHT;
                let height = ((world_bottom - camera.y + 60.) as f32 * scale).clamp(400., 32768.);
                let (area, _) =
                    ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
                if self
                    .scenarios
                    .drag
                    .as_ref()
                    .is_some_and(|d| d.canvas_origin != area.min)
                {
                    self.scenarios.drag = None;
                    preview = false;
                    for o in &mut occurrences {
                        o.geometry = diagram.occurrences[&o.id].geometry.clone();
                    }
                    self.scenarios.note =
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
                let bottom = area.top() + ((world_bottom - camera.y) as f32) * scale;
                for rect in rects.values() {
                    dashed(
                        ui.painter(),
                        egui::pos2(rect.center().x, rect.bottom()),
                        egui::pos2(rect.center().x, bottom),
                        Stroke::new(1., LINE),
                    );
                }
                if let Some(a) = source_scenario {
                    let left = rects
                        .values()
                        .map(|r| r.left())
                        .fold(area.left() + 20., f32::min);
                    let right = rects
                        .values()
                        .map(|r| r.right())
                        .fold(area.left() + 300., f32::max);
                    let mut alt_top = None;
                    for (i, row) in projection.rows.iter().enumerate() {
                        let y = area.top()
                            + ((base_y + (i + 1) as f64 * va::ROW_HEIGHT - camera.y) as f32)
                                * scale;
                        match row {
                            va::Row::AlternativeStart { alternative } => {
                                alt_top = Some(y - 20. * scale);
                                let alt = &a.alternatives[alternative];
                                let text = format!("alt #{} · {}", alternative.0, alt.name);
                                paint_label(
                                    ui,
                                    Id(alternative.0),
                                    text,
                                    Rect::from_min_max(
                                        egui::pos2(left + 5., y - 18. * scale),
                                        egui::pos2(right - 5., y + 10. * scale),
                                    ),
                                    scale,
                                    &mut labels,
                                );
                            }
                            va::Row::OperandStart {
                                alternative,
                                operand,
                            } => {
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(left, y - 30. * scale),
                                        egui::pos2(right, y - 30. * scale),
                                    ],
                                    Stroke::new(1., LINE),
                                );
                                let op = &a.alternatives[alternative].operands[operand];
                                paint_label(
                                    ui,
                                    *operand,
                                    format!("Operand #{} [{}]", operand.0, op.guard),
                                    Rect::from_min_max(
                                        egui::pos2(left + 5., y - 20. * scale),
                                        egui::pos2(right - 5., y + 15. * scale),
                                    ),
                                    scale,
                                    &mut labels,
                                );
                            }
                            va::Row::AlternativeEnd { .. } => {
                                if let Some(top) = alt_top.take() {
                                    ui.painter().rect_stroke(
                                        Rect::from_min_max(
                                            egui::pos2(left, top),
                                            egui::pos2(right, y),
                                        ),
                                        0.,
                                        Stroke::new(1., LINE),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                            va::Row::Message { message } => {
                                if let Some(route) =
                                    projection.routes.iter().find(|r| r.message == *message)
                                {
                                    if let (Some(sr), Some(tr)) =
                                        (rects.get(&route.source), rects.get(&route.target))
                                    {
                                        let start = egui::pos2(sr.center().x, y);
                                        let mut end = egui::pos2(tr.center().x, y);
                                        let final_start = if route.source == route.target {
                                            let turn = egui::pos2(start.x + 60. * scale, y);
                                            let down = egui::pos2(turn.x, y + 24. * scale);
                                            ui.painter().line_segment(
                                                [start, turn],
                                                Stroke::new(1.5, ACCENT),
                                            );
                                            ui.painter().line_segment(
                                                [turn, down],
                                                Stroke::new(1.5, ACCENT),
                                            );
                                            end.y = down.y;
                                            down
                                        } else {
                                            start
                                        };
                                        if route.kind == MessageKind::Reply {
                                            dashed(
                                                ui.painter(),
                                                final_start,
                                                end,
                                                Stroke::new(1.5, ACCENT),
                                            );
                                            open_head(
                                                ui.painter(),
                                                final_start,
                                                end,
                                                Stroke::new(1.5, ACCENT),
                                            );
                                        } else if route.kind == MessageKind::Asynchronous {
                                            ui.painter().line_segment(
                                                [final_start, end],
                                                Stroke::new(1.5, ACCENT),
                                            );
                                            open_head(
                                                ui.painter(),
                                                final_start,
                                                end,
                                                Stroke::new(1.5, ACCENT),
                                            );
                                        } else {
                                            paint_flow_arrow(
                                                ui.painter(),
                                                final_start,
                                                end,
                                                Stroke::new(1.5, ACCENT),
                                            );
                                        }
                                        let m = &a.messages[message];
                                        let text = format!(
                                            "{} · #{} · {} · {:?}",
                                            m.order, m.id.0, m.name, m.kind
                                        );
                                        paint_label(
                                            ui,
                                            *message,
                                            text,
                                            Rect::from_min_max(
                                                egui::pos2(
                                                    start.x.min(end.x) + 5.,
                                                    y - 32. * scale,
                                                ),
                                                egui::pos2(
                                                    (start.x.max(end.x) - 5.)
                                                        .max(start.x.min(end.x) + 100.),
                                                    y - 4. * scale,
                                                ),
                                            ),
                                            scale,
                                            &mut labels,
                                        );
                                        rendered_messages.push(*message);
                                        endpoints.push((
                                            *message,
                                            [start.x, start.y],
                                            [end.x, end.y],
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                for o in &occurrences {
                    let rect = rects[&o.id];
                    let selected = self.scenarios.selected == Some((view, o.id));
                    let stroke = Stroke::new(
                        if selected { 2.5 } else { 1. },
                        if selected { ACCENT } else { LINE },
                    );
                    ui.painter()
                        .rect(rect, 7., PAPER, stroke, egui::StrokeKind::Inside);
                    let label = source_scenario
                        .and_then(|a| a.lifelines.get(&o.element))
                        .map_or(String::new(), |l| {
                            format!(
                                "{} · participant {:?} #{}{}",
                                l.name,
                                l.expected,
                                l.participant.0,
                                if projection.unresolved_lifelines.contains(&l.id) {
                                    " · UNRESOLVED"
                                } else {
                                    ""
                                }
                            )
                        });
                    let text = format!("Lifeline #{}\n{}", o.element.0, label);
                    let galley = ui.painter().layout(
                        text,
                        FontId::proportional(13. * scale),
                        INK,
                        (rect.width() - 20. * scale).max(1.),
                    );
                    ui.painter().with_clip_rect(rect).galley(
                        rect.min + egui::vec2(10. * scale, 10. * scale),
                        galley,
                        INK,
                    );
                    let r = ui
                        .interact(
                            rect,
                            ui.id().with(("scenario-occurrence", o.id.get())),
                            egui::Sense::click_and_drag(),
                        )
                        .on_hover_text(format!(
                            "Scenario #{} · element #{} · {label}",
                            diagram.scenario.0, o.element.0
                        ));
                    self.bound(format!("scenario-occurrence:{}", o.id.get()), &r);
                    if r.drag_started() && ready && !o.pinned {
                        if let Some(pointer) = r
                            .ctx
                            .input(|i| i.pointer.press_origin())
                            .or_else(|| r.interact_pointer_pos())
                        {
                            self.scenarios.drag = Some(Drag {
                                view,
                                occurrence: o.id,
                                stamp: stamp.clone(),
                                geometry: o.geometry.clone(),
                                start: pointer,
                                viewport: self.context.content_rect(),
                                canvas_origin: area.min,
                            });
                            self.scenarios.selected = Some((view, o.id));
                        }
                    }
                    if r.drag_stopped() {
                        if let Some(d) = self.scenarios.drag.take() {
                            if d.view == view && d.occurrence == o.id {
                                let pointer =
                                    r.ctx.input(|i| i.pointer.latest_pos()).unwrap_or(d.start);
                                let mut geometry = d.geometry;
                                geometry.x += (pointer.x - d.start.x) as f64 / camera.zoom;
                                geometry.y += (pointer.y - d.start.y) as f64 / camera.zoom;
                                let result = self.sc_layout(
                                    &d.stamp,
                                    va::Layout::Move {
                                        view,
                                        occurrence: o.id,
                                        geometry,
                                    },
                                );
                                self.scenarios.note = result
                                    .map(|_| {
                                        "Diagram moved once; native scenario data unchanged.".into()
                                    })
                                    .unwrap_or_else(|e| e);
                            }
                        }
                    }
                    if r.clicked() && ready {
                        self.scenarios.selected = Some((view, o.id));
                        select = Some(o.element);
                    }
                }
            });
        self.scenarios.rendered = Some(Rendered {
            view,
            stamp: stamp.clone(),
            occurrences: occurrences
                .iter()
                .map(|o| (o.id, o.element, o.geometry.clone()))
                .collect(),
            messages: rendered_messages.clone(),
            rows: serde_json::to_value(&projection.rows).unwrap(),
            endpoints,
            labels,
            preview,
        });
        ui.separator();
        ui.label(format!(
            "Sequence ledger · {} arrows shown · {} without unique resolved visible endpoints",
            rendered_messages.len(),
            projection.unshown_messages
        ));
        if let Some(a) = source_scenario {
            for row in &projection.rows {
                let id = match row {
                    va::Row::Message { message } => *message,
                    va::Row::AlternativeStart { alternative }
                    | va::Row::AlternativeEnd { alternative } => *alternative,
                    va::Row::OperandStart { operand, .. } => *operand,
                };
                let f = super::scenario_editor::accepted_form(a, id).unwrap();
                let r = ui.button(format!(
                    "Select #{} · {}",
                    id.0,
                    serde_json::to_string(&f).unwrap()
                ));
                self.bound(format!("scenario-row:{}", id.0), &r);
                if r.clicked() && ready {
                    select = Some(id);
                }
            }
            ui.collapsing(
                "All lifelines (including hidden, detached and unshown)",
                |ui| {
                    for l in a.lifelines.values() {
                        ui.horizontal_wrapped(|ui| {
                            let r = ui.add_enabled(
                                ready,
                                egui::Button::new(format!(
                                    "Edit lifeline #{} · {}",
                                    l.id.0, l.name
                                )),
                            );
                            self.bound(format!("scenario-lifeline:{}", l.id.0), &r);
                            if r.clicked() {
                                select = Some(l.id);
                            }
                            let r = ui.add_enabled(ready, egui::Button::new("Reuse lifeline"));
                            self.bound(format!("scenario-reuse:{}", l.id.0), &r);
                            if r.clicked() {
                                change = Some(va::Layout::Reuse {
                                    view,
                                    element: l.id,
                                    geometry: nv::Geometry::at(25., 24.),
                                });
                            }
                        });
                    }
                },
            );
        }
        if let Some(redo) = history {
            let result = if self.dock_blocked_without_scenario() || self.sc_pending() {
                Err("Finish pending work before diagram history.".into())
            } else {
                self.views
                    .views
                    .undo(&self.session.editor, &stamp, redo)
                    .map(|_| ())
            };
            self.views.reconcile();
            self.scenarios.note = result
                .map(|_| "Diagram history restored; native data unchanged.".into())
                .unwrap_or_else(|e| e);
        } else if let Some(action) = change {
            self.scenarios.note = self
                .sc_layout(&stamp, action)
                .map(|_| "Diagram presentation updated; native scenario data unchanged.".into())
                .unwrap_or_else(|e| e);
        } else if let Some(id) = select {
            if let Err(error) = self.sc_select_element(&stamp, view, id) {
                self.scenarios.note = error;
            }
        }
        true
    }
}
fn dashed(p: &egui::Painter, start: egui::Pos2, end: egui::Pos2, stroke: Stroke) {
    let delta = end - start;
    let len = delta.length();
    if len < 0.1 {
        return;
    }
    let unit = delta / len;
    let mut x = 0.;
    while x < len {
        p.line_segment([start + unit * x, start + unit * (x + 6.).min(len)], stroke);
        x += 10.;
    }
}
fn open_head(p: &egui::Painter, start: egui::Pos2, end: egui::Pos2, stroke: Stroke) {
    let delta = end - start;
    let len = delta.length();
    if len < 0.1 {
        return;
    }
    let unit = delta / len;
    let back = end - unit * len.min(10.);
    let side = egui::vec2(-unit.y, unit.x) * len.min(5.);
    p.line_segment([back + side, end], stroke);
    p.line_segment([back - side, end], stroke);
}
fn paint_label(
    ui: &egui::Ui,
    id: Id,
    text: String,
    rect: Rect,
    scale: f32,
    labels: &mut Vec<(Id, [f32; 4], String)>,
) {
    // Single-line, clipped preview only. Exact full text remains in the selectable ledger/editor.
    let preview = text.replace(['\n', '\t'], " ");
    let galley = ui.painter().layout_no_wrap(
        preview,
        FontId::proportional((12. * scale).clamp(9., 16.)),
        ACCENT,
    );
    let bounds = Rect::from_min_size(
        rect.min,
        egui::vec2(
            galley.size().x.min(rect.width().max(1.)),
            galley.size().y.min(rect.height().max(1.)),
        ),
    );
    ui.painter().rect_filled(bounds.expand(2.), 2., PAPER);
    ui.painter()
        .with_clip_rect(rect)
        .galley(rect.min, galley, ACCENT);
    ui.interact(
        rect,
        ui.id().with(("scenario-label", id.0)),
        egui::Sense::hover(),
    )
    .on_hover_text(&text);
    labels.push((
        id,
        [bounds.left(), bounds.top(), bounds.right(), bounds.bottom()],
        text,
    ));
}

//! App-owned UI over the bounded, provenance-preserving Stage1 contract.
use craft_ui::{buttons::{ButtonVisuals, IconButton, IconButtonStyle}, theme::WidgetPalette};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use mbse_model::{
    stage1::{Command, CreationDraft, RawKey, Record, RecordId, RecordKind, SubjectRef, Validity},
    stage1_calc::{self as calc, CalculationId, Definition, Limits, QuantityType, UnitId},
    stage1_graph::{self, EdgeId, GraphEdge},
    stage1_io::{self, Session},
};
use std::{collections::BTreeMap, path::PathBuf};

const INK: Color32 = Color32::from_rgb(36, 48, 63);
const ACCENT: Color32 = Color32::from_rgb(18, 100, 119);
const PAPER: Color32 = Color32::from_rgb(252, 251, 247);
const LINE: Color32 = Color32::from_rgb(197, 207, 209);
const ERROR: Color32 = Color32::from_rgb(160, 42, 55);

#[derive(Clone)]
enum Load { New, Open, Recover, Import, Quit }

pub struct Workspace {
    session: Session,
    path: String,
    feedback: String,
    error: bool,
    creation: Option<(RecordKind, RecordId, String)>,
    relation: Option<EdgeId>,
    popout: Option<RecordId>,
    pending: Option<Load>,
    capture: Option<PathBuf>,
    frames: u32,
    screenshot_requested: bool,
    zoom: f32,
}
impl Workspace {
    pub fn new(session: Session, capture: Option<PathBuf>, ctx: &egui::Context) -> Self {
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::light();
        style.visuals.panel_fill = PAPER;
        style.visuals.window_fill = PAPER;
        style.visuals.override_text_color = Some(INK);
        // Real ArtCraft shared palette API, pinned and licensed in vendor/.
        WidgetPalette { noninteractive: PAPER, inactive: Color32::from_rgb(236, 241, 240),
            hovered: Color32::from_rgb(220, 234, 232), active: Color32::from_rgb(202, 226, 224),
            open: Color32::from_rgb(220, 234, 232) }
            .apply_to(&mut style.visuals.widgets, egui::CornerRadius::same(5), Stroke::new(1., LINE));
        style.spacing.item_spacing = Vec2::new(9., 8.);
        ctx.set_style(style);
        let path = session.path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        Self { session, path, feedback: "Select a model card to edit its fields. Model values and unfinished text remain separate.".into(), error: false,
            creation: None, relation: None, popout: None, pending: None, capture, frames: 0, screenshot_requested: false, zoom: 1. }
    }
    fn result(&mut self, result: Result<String, String>) {
        match result { Ok(message) => { self.feedback = message; self.error = false; }, Err(error) => { self.feedback = error; self.error = true; } }
    }
    fn command(&mut self, command: Command) {
        let result = self.session.editor.command(&self.session.editor.freshness(), command);
        self.result(result.map(|outcome| {
            if let Some(id) = outcome.created { let _ = self.session.editor.select(id); }
            if outcome.accepted { "Model updated. Save includes accepted values and retained drafts.".into() }
            else { "Draft retained; last accepted model value is unchanged.".into() }
        }));
    }
    fn field(&mut self, ui: &mut egui::Ui, record: &Record, field: &str, label: &str, multiline: bool) {
        let key = RawKey { record: record.id, field: field.into(), item: None };
        if !self.session.editor.field_available(&key) { return; }
        ui.label(RichText::new(label).strong());
        let mut text = self.session.editor.display_text(&key);
        let input = if multiline { egui::TextEdit::multiline(&mut text).desired_rows(2) } else { egui::TextEdit::singleline(&mut text) };
        let response = ui.add(input.id_salt((record.id.0, field)).desired_width(f32::INFINITY));
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
        if response.changed() { self.command(Command::EditText { key: key.clone(), text }); }
        if response.lost_focus() { self.session.editor.finish_field(); }
        if let Some(raw) = self.session.editor.raw(&key) {
            if raw.validity != Validity::Valid {
                ui.colored_label(ERROR, format!("{:?} draft retained", raw.validity));
                ui.label(RichText::new(format!("Last accepted: {}", if field == "criterion" { record.number(field).map(|v| v.to_string()).unwrap_or_else(|| "absent".into()) } else { record.text(field).into() })).small());
            }
        }
    }
    fn save(&mut self) {
        let path = PathBuf::from(self.path.trim());
        if self.path.trim().is_empty() { self.result(Err("Enter a destination path before saving.".into())); return; }
        let result = self.session.begin_save(&path).map(|job| {
            let result = stage1_io::execute_write(job);
            let error = result.error().map(|e| format!("{e:?}"));
            self.session.complete(result);
            error.map_or(Ok("Saved accepted values and raw drafts.".into()), Err)
        }).and_then(|r| r);
        self.result(result);
    }
    fn recovery(&mut self) -> Result<String, String> {
        let job = self.session.begin_recovery()?;
        let result = stage1_io::execute_write(job);
        let error = result.error().map(|e| format!("{e:?}"));
        self.session.complete(result);
        if let Some(error) = error { return Err(error); }
        Ok(format!("Recovery copy: {}", self.session.last_recovery().map(|p| p.display().to_string()).unwrap_or_default()))
    }
    fn request_load(&mut self, action: Load) {
        if self.session.is_dirty() { self.pending = Some(action); } else { self.load(action); }
    }
    fn load(&mut self, action: Load) {
        if matches!(action, Load::Quit) { return; }
        let root = self.session.state_root.clone();
        let result = match action {
            Load::New => Session::new(&root),
            Load::Open => Session::open(&root, &PathBuf::from(self.path.trim())),
            Load::Recover => Session::restore(&root, &PathBuf::from(self.path.trim())),
            Load::Import => Session::import_sysml(&root, &PathBuf::from(self.path.trim())),
            Load::Quit => unreachable!(),
        };
        match result {
            Ok(session) => {
                self.session = session; self.creation = None; self.relation = None; self.popout = None;
                self.path = self.session.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                self.result(Ok("Model opened. Imported and recovered documents are independent drafts.".into()));
            }
            Err(error) => self.result(Err(error)),
        }
    }
    fn toolbar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("workspace_toolbar").show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("MBSE-Craft"); ui.label("Model workspace"); ui.separator();
                for (label, action) in [("New", Load::New), ("Open", Load::Open), ("Recover", Load::Recover), ("Import SysML", Load::Import)] {
                    if ui.button(label).clicked() { self.request_load(action); }
                }
                if ui.button("Save").clicked() { self.save(); }
                if ui.button("Recovery copy").clicked() { let result = self.recovery(); self.result(result); }
                if ui.button("Export SysML copy").clicked() {
                    let result = self.session.begin_export_sysml(&PathBuf::from(self.path.trim())).and_then(|job| self.session.complete_export(stage1_io::execute_export(job))).map(|_| "Exported restricted SysML copy.".into());
                    self.result(result);
                }
                ui.separator();
                // Real ArtCraft IconButton controls with accessible action labels.
                let visual = ButtonVisuals { fill: Color32::from_rgb(231, 239, 238), stroke: Stroke::new(1., LINE), icon: ACCENT };
                let style = IconButtonStyle { normal: visual, hovered: visual, pressed: Some(visual), selected: visual,
                    corner_radius: egui::CornerRadius::same(4), focus_stroke: Stroke::new(2., ACCENT) };
                for (label, undo, enabled) in [("Undo", true, self.session.editor.can_undo()), ("Redo", false, self.session.editor.can_redo())] {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = IconButton::new(label, Vec2::splat(30.), &style).show(ui, |ui, rect, tint| {
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if undo { "↶" } else { "↷" }, FontId::proportional(20.), tint);
                        });
                        if response.clicked() { self.history(undo); }
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("File path");
                ui.add(egui::TextEdit::singleline(&mut self.path).id_salt("document-path").desired_width(ui.available_width()).hint_text("/workspace/…/model.kernspan (or source/recovery path for Open / Recover)"));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{:?} · model r{} · raw v{}", self.session.status(), self.session.editor.snapshot.model_revision, self.session.editor.snapshot.raw_version));
                if self.session.recovery_current() { ui.colored_label(ACCENT, "Current recovery copy"); }
                ui.label(RichText::new("App-owned model · restricted SysML subset").small());
            });
        });
    }
    fn history(&mut self, undo: bool) {
        let freshness = self.session.editor.freshness();
        let result = if undo { self.session.editor.undo_model(&freshness) } else { self.session.editor.redo_model(&freshness) };
        self.result(result.map(|changed| if changed { "Model history applied. Raw drafts remain separately retained.".into() } else { "No model history available.".into() }));
    }
    fn sidebar(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("model_navigation").default_size(338.).size_range(290. ..=420.).resizable(true).show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Model");
                let records: Vec<_> = self.session.editor.snapshot.records.values().cloned().collect();
                let selected = self.session.editor.selected;
                egui::CollapsingHeader::new(format!("Records · {}", records.len())).default_open(true).show(ui, |ui| {
                    for record in &records {
                        if ui.selectable_label(selected == Some(record.id), format!("{} · {}", record.kind.label(), if record.name.is_empty() { "Unnamed" } else { &record.name })).clicked() { let _ = self.session.editor.select(record.id); }
                    }
                });
                ui.separator();
                if let Some(record) = selected.and_then(|id| records.iter().find(|r| r.id == id)).cloned() {
                    ui.label(RichText::new(format!("{} #{}", record.kind.label(), record.id.0)).color(ACCENT));
                    self.inspector(ui, &record, &records);
                }
                ui.separator(); ui.label(RichText::new("Add a typed record").strong());
                ui.horizontal_wrapped(|ui| {
                    for kind in [RecordKind::Mission, RecordKind::Capability, RecordKind::Objective, RecordKind::Measure, RecordKind::Actor, RecordKind::Stakeholder, RecordKind::Environment, RecordKind::Organization, RecordKind::Expectation] {
                        let owner = if kind == RecordKind::Measure { selected.filter(|id| records.iter().any(|r| r.id == *id && r.kind == RecordKind::Objective)) }
                            else if kind == RecordKind::Expectation { selected.filter(|id| records.iter().any(|r| r.id == *id && r.kind == RecordKind::Stakeholder)) }
                            else { selected.and_then(|id| records.iter().find(|r| r.id == id).map(|r| r.scope)) };
                        let label = format!("Add {}", kind.label());
                        if ui.add_enabled(owner.is_some(), egui::Button::new(label)).on_hover_text(if kind == RecordKind::Measure { "Select the Objective this Measure assesses first." } else if kind == RecordKind::Expectation { "Select the Stakeholder expressing this expectation first." } else { "Create a record in this System." }).clicked() {
                            let owner = owner.expect("enabled owner");
                            let draft_id = format!("mbse-ui-add-{}-{}", kind.label(), owner.0);
                            let text = self.session.editor.snapshot.creation_drafts.get(&draft_id).and_then(|d| d.fields.get("name")).cloned().unwrap_or_default();
                            self.creation = Some((kind, owner, text));
                        }
                    }
                });
                ui.label(RichText::new("Select an Objective to Add Measure; select a Stakeholder to Add Expectation.").small());
            });
        });
    }
    fn inspector(&mut self, ui: &mut egui::Ui, record: &Record, records: &[Record]) {
        self.field(ui, record, "name", "Name", false);
        match record.kind {
            RecordKind::System => self.field(ui, record, "purpose", "Purpose", true),
            RecordKind::Mission => { ui.label("Mission: intended accomplishment. Naming does not infer a goal, owner or classifier."); },
            RecordKind::Objective => {
                self.field(ui, record, "outcome", "Desired outcome", true);
                ui.label(RichText::new("Subject").strong());
                egui::ComboBox::from_id_salt(("objective-subject", record.id.0)).selected_text(record.subject().map(|s| format!("{s:?}")).unwrap_or_else(|| "Choose typed subject…".into())).show_ui(ui, |ui| {
                    if ui.selectable_label(record.subject().is_none(), "No subject").clicked() { self.command(Command::SetSubject { objective: record.id, subject: None }); }
                    for target in records.iter().filter(|r| r.scope == record.scope) {
                        let subject = match target.kind { RecordKind::System => Some(SubjectRef::System(target.id)), RecordKind::Mission => Some(SubjectRef::Mission(target.id)), RecordKind::Capability => Some(SubjectRef::Capability(target.id)), _ => None };
                        if let Some(subject) = subject {
                            if ui.selectable_label(record.subject() == Some(subject), format!("{} · {} #{}", target.kind.label(), target.name, target.id.0)).clicked() { self.command(Command::SetSubject { objective: record.id, subject: Some(subject) }); }
                        }
                    }
                });
            }
            RecordKind::Capability => {
                ui.label(RichText::new("Contributes to missions").strong());
                for mission in records.iter().filter(|r| r.kind == RecordKind::Mission && r.scope == record.scope) {
                    let mut linked = record.links().contains(&mission.id);
                    if ui.checkbox(&mut linked, format!("{} #{}", mission.name, mission.id.0)).changed() { self.command(Command::ToggleMission { capability: record.id, mission: mission.id }); }
                }
            }
            RecordKind::Measure => {
                ui.label(RichText::new("Quantity and unit").strong());
                egui::ComboBox::from_id_salt(("measure-unit", record.id.0)).selected_text(format!("{} · {}", record.text("quantity"), record.text("unit"))).show_ui(ui, |ui| {
                    for (quantity, unit, label) in [("dimensionless", "percent", "Ratio · %"), ("dimensionless", "one", "Scalar · 1"), ("length", "metre", "Length · m")] {
                        if ui.selectable_label(record.text("unit") == unit, label).clicked() { self.command(Command::SetQuantity { measure: record.id, quantity: quantity.into(), unit: unit.into() }); }
                    }
                });
                self.field(ui, record, "criterion", "Target criterion", false);
                ui.label(format!("Accepted criterion: {} {}", record.number("criterion").map(|v| v.to_string()).unwrap_or_else(|| "absent".into()), record.text("unit")));
                ui.collapsing("For Objective", |ui| {
                    for objective in records.iter().filter(|r| r.kind == RecordKind::Objective && r.scope == record.scope) {
                        if ui.selectable_label(record.fields.get("forObjective").and_then(serde_json::Value::as_u64) == Some(objective.id.0), format!("{} #{}", objective.name, objective.id.0)).clicked() { self.command(Command::SetForObjective { measure: record.id, objective: objective.id }); }
                    }
                });
                ui.collapsing("Evidence and context", |ui| { for (field, label) in [("assessmentMethod", "Assessment method"), ("conditions", "Conditions"), ("uncertainty", "Uncertainty")] { self.field(ui, record, field, label, true); } });
                self.calculation(ui, record);
                ui.label(RichText::new("Custom Measure. MOE/MOP metadata does not imply SysML classifier equivalence.").small());
            }
            RecordKind::Stakeholder => self.field(ui, record, "interest", "Interest", true),
            RecordKind::Expectation => self.field(ui, record, "text", "Expectation", true),
            _ => { ui.label("Typed record; additional source properties remain preserved in the native file."); },
        }
    }
    fn calculation(&mut self, ui: &mut egui::Ui, record: &Record) {
        ui.separator(); ui.label(RichText::new("Local calculation").strong());
        let definitions = self.session.editor.snapshot.calculations().unwrap_or_default();
        if let Some(definition) = definitions.values().find(|d| d.owner == record.id) {
            self.field(ui, record, "calc.expression", "Expression", false);
            for (name, input) in &definition.bindings {
                if matches!(input, calc::Input::Literal(_)) { self.field(ui, record, &format!("calc.input.{name}"), &format!("Input {name}"), false); }
            }
            ui.label(format!("Output unit: {}", calc::unit_label(definition.output.unit)));
            if ui.button("Remove calculation").clicked() { self.command(Command::PutCalculation { measure: record.id, definition: None }); }
        } else {
            ui.label("Create a bounded expression in the Measure's unit; numeric literals are dimensionless. Unit literals use syntax such as 3[m].");
            if ui.button("Add calculation").clicked() {
                let unit = match record.text("unit") { "percent" => UnitId::Percent, "metre" => UnitId::Metre, _ => UnitId::ScalarOne };
                let expression = calc::parse("0", &Limits::default()).expect("constant expression");
                self.command(Command::PutCalculation { measure: record.id, definition: Some(Definition { id: CalculationId(record.id.0), owner: record.id, bindings: BTreeMap::new(), expression, output: QuantityType { kind: calc::unit_kind(unit), unit } }) });
            }
        }
        match self.session.editor.evaluate_calculations() {
            Ok(receipt) => if let Some(outcome) = receipt.outcomes.get(&CalculationId(record.id.0)) {
                match outcome {
                    calc::Outcome::Value(value) => { ui.colored_label(ACCENT, format!("Derived result: {} {}", value.value, calc::unit_label(value.quantity.unit))); },
                    calc::Outcome::Failure(error) => { ui.colored_label(ERROR, format!("{:?}: {}", error.code, error.message)); },
                }
                ui.label(RichText::new("Derived result is separate from criterion and observations.").small());
            },
            Err(error) => { ui.colored_label(ERROR, error); },
        }
    }
    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.horizontal(|ui| { ui.heading("System canvas"); ui.label("Select to edit · Details opens a popout"); ui.add(egui::Slider::new(&mut self.zoom, 0.65..=1.4).text("Scale")); });
            let records: Vec<_> = self.session.editor.snapshot.records.values().cloned().collect();
            let graph = match stage1_graph::derive_graph(&self.session.editor.snapshot) { Ok(graph) => graph, Err(error) => { ui.colored_label(ERROR, error); return; } };
            let canvas_height = (ui.available_height() * 0.60).max(260.);
            egui::ScrollArea::both().id_salt("primary-canvas").max_height(canvas_height).auto_shrink([false,false]).show(ui, |ui| {
                let scale = self.zoom;
                let size = Vec2::new(1170. * scale, (records.len().div_ceil(3) as f32 * 155. + 50.).max(350.) * scale);
                let (area, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let rects: BTreeMap<_, _> = records.iter().enumerate().map(|(index, r)| {
                    let min = area.min + Vec2::new(25. + (index % 3) as f32 * 385., 24. + (index / 3) as f32 * 155.) * scale;
                    (r.id, Rect::from_min_size(min, Vec2::new(330., 112.) * scale))
                }).collect();
                for (index, edge) in graph.edges.iter().enumerate() {
                    let (Some(source), Some(target)) = (rects.get(&edge.id.source), rects.get(&edge.id.target)) else { continue; };
                    let a = source.center_bottom(); let b = target.center_top();
                    let bend = Pos2::new(a.x + (index % 5) as f32 * 8., a.y + 14. * scale);
                    ui.painter().line_segment([a,bend], Stroke::new(1., ACCENT));
                    ui.painter().arrow(bend, b-bend, Stroke::new(1., ACCENT));
                }
                for record in &records {
                    let rect = rects[&record.id];
                    let selected = self.session.editor.selected == Some(record.id);
                    ui.painter().rect(rect, 7., PAPER, Stroke::new(if selected { 2.5 } else { 1. }, if selected { ACCENT } else { LINE }), egui::StrokeKind::Inside);
                    ui.painter().text(rect.min + Vec2::new(14., 13.) * scale, egui::Align2::LEFT_TOP, format!("{}  #{}", record.kind.label(), record.id.0), FontId::proportional(12. * scale), ACCENT);
                    let name = if record.name.is_empty() { "Name your system" } else { &record.name };
                    let galley = ui.painter().layout(name.into(), FontId::proportional(17. * scale), INK, rect.width()-28.*scale);
                    ui.painter().with_clip_rect(rect.shrink(5.)).galley(rect.min + Vec2::new(14., 36.)*scale, galley, INK);
                    let select_rect = Rect::from_min_max(rect.min, rect.max - Vec2::new(0., 28.*scale));
                    let response = ui.interact(select_rect, ui.id().with(("record",record.id.0)), egui::Sense::click());
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{} {} #{}", record.kind.label(), name, record.id.0)));
                    if response.clicked() { let _ = self.session.editor.select(record.id); }
                    let details = Rect::from_min_size(rect.left_bottom() + Vec2::new(12.*scale,-27.*scale), Vec2::new(90.,22.)*scale);
                    if ui.put(details, egui::Button::new("Details…")).clicked() { self.popout = Some(record.id); }
                }
            });
            ui.separator(); ui.label(RichText::new(format!("Typed relations · {} · all links remain listed regardless of canvas density", graph.edges.len())).strong());
            egui::ScrollArea::vertical().id_salt("relation-ledger").show(ui, |ui| {
                for edge in &graph.edges {
                    if ui.selectable_label(self.relation == Some(edge.id), relation_label(edge, &records)).clicked() { self.relation = Some(edge.id); }
                }
                if graph.edges.is_empty() { ui.label("Add records and typed links to populate the relation ledger."); }
            });
        });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some((kind, owner, mut name)) = self.creation.clone() {
            let mut open = true; let mut create = false;
            egui::Window::new(format!("Add {}", kind.label())).id(egui::Id::new("create-record")).open(&mut open).collapsible(false).show(ctx, |ui| {
                ui.label(format!("Owner #{} · explicit typed scope", owner.0));
                ui.label("Name");
                if ui.add(egui::TextEdit::singleline(&mut name).id_salt("creation-name").desired_width(350.)).changed() {
                    self.command(Command::PutCreationDraft { id: format!("mbse-ui-add-{}-{}",kind.label(),owner.0), draft: CreationDraft { kind, owner, fields: BTreeMap::from([("name".into(), name.clone())]) } });
                }
                ui.label("Unfinished text is retained when you close this panel. Reopen Add to continue.");
                if ui.add_enabled(!name.trim().is_empty(), egui::Button::new(format!("Create {}",kind.label()))).clicked() { create = true; }
            });
            self.creation = if open && !create { Some((kind,owner,name.clone())) } else { None };
            if create { self.command(Command::Add { kind,owner,name }); }
        }
        if let Some(edge) = self.relation {
            let mut open = true;
            egui::Window::new("Typed relation").open(&mut open).show(ctx, |ui| {
                ui.label(format!("{:?}", edge.role));
                ui.label(format!("Source #{} → Target #{}", edge.source.0, edge.target.0));
                for (label,id) in [("Select source",edge.source),("Select target",edge.target)] { if ui.button(label).clicked() { let _ = self.session.editor.select(id); } }
                ui.label("This view displays authored references. Canvas proximity creates no semantic relation.");
            });
            if !open { self.relation = None; }
        }
        if let Some(id) = self.popout {
            let mut open = true;
            if let Some(record) = self.session.editor.snapshot.records.get(&id).cloned() {
                egui::Window::new(format!("{} #{}",record.kind.label(),id.0)).open(&mut open).show(ctx, |ui| {
                    ui.heading(&record.name); ui.label(format!("System scope #{}",record.scope.0));
                    if ui.button("Edit in left controls").clicked() { let _ = self.session.editor.select(id); }
                    ui.collapsing("Preserved record properties", |ui| { ui.monospace(serde_json::to_string_pretty(&record.fields).unwrap_or_default()); });
                });
            } else { open = false; }
            if !open { self.popout = None; }
        }
        if let Some(action) = self.pending.clone() {
            egui::Window::new("Preserve current work").collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label("The current model has unsaved accepted values or retained drafts.");
                ui.label("Save to the file path, or create a durable recovery copy before continuing.");
                if ui.button("Save and continue").clicked() {
                    self.save();
                    if !self.session.is_dirty() { self.pending = None; if matches!(action,Load::Quit) { ctx.send_viewport_cmd(egui::ViewportCommand::Close); } else { self.load(action.clone()); } }
                }
                if ui.button("Keep recovery and continue").clicked() {
                    let result = self.recovery(); let ok = result.is_ok(); self.result(result);
                    if ok { self.pending = None; if matches!(action,Load::Quit) { ctx.send_viewport_cmd(egui::ViewportCommand::Close); } else { self.load(action.clone()); } }
                }
                if ui.button("Cancel").clicked() { self.pending = None; }
            });
        }
    }
    fn capture(&mut self, ctx: &egui::Context) {
        if self.capture.is_none() { return; }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                if !self.screenshot_requested { continue; }
                let path = self.capture.take().expect("capture configured");
                let result = (|| -> Result<String,String> {
                    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p|p.to_array()).collect();
                    let buffer = image::RgbaImage::from_raw(image.width() as u32,image.height() as u32,pixels).ok_or("Invalid image dimensions")?;
                    let file = std::fs::OpenOptions::new().create_new(true).write(true).open(&path).map_err(|e|e.to_string())?;
                    image::DynamicImage::ImageRgba8(buffer).write_to(&mut std::io::BufWriter::new(file),image::ImageFormat::Png).map_err(|e|e.to_string())?;
                    Ok(format!("Native frame captured: {}",path.display()))
                })();
                self.result(result); eprintln!("{}",self.feedback);
            }
        }
        if self.frames > 4 && !self.screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default())); self.screenshot_requested = true;
        }
        ctx.request_repaint();
    }
}
fn relation_label(edge: &GraphEdge, records: &[Record]) -> String {
    let name = |id| records.iter().find(|r|r.id==id).map(|r|format!("{} · {} #{}",r.kind.label(),r.name,id.0)).unwrap_or_else(||format!("Unavailable #{}",id.0));
    format!("{}  — {} →  {}{}",name(edge.id.source),edge.label,name(edge.id.target),if edge.binding_names.is_empty() { String::new() } else { format!(" [{}]",edge.binding_names.join(", ")) })
}
impl eframe::App for Workspace {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone(); self.frames = self.frames.saturating_add(1);
        if ctx.input(|i|i.viewport().close_requested()) && self.session.is_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose); self.pending = Some(Load::Quit);
        }
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::COMMAND,egui::Key::S)) { self.save(); }
        egui::Panel::bottom("feedback").show(root, |ui| { ui.colored_label(if self.error {ERROR} else {INK},&self.feedback); });
        self.toolbar(root); self.sidebar(root); self.canvas(root); self.dialogs(&ctx); self.capture(&ctx);
    }
}

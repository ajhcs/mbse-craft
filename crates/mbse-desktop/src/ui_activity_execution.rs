//! Native opt-in controls over the one finite activity execution authority.
use super::*;
use mbse_model::types::activities::execution::{Execution, Status};
pub(super) struct Panel {
    pub live: Option<Execution>,
    pub automatic: bool,
    pub next_tick: Option<Instant>,
    pub budget: String,
    pub note: String,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            live: None,
            automatic: false,
            next_tick: None,
            budget: "128".into(),
            note: String::new(),
        }
    }
}
impl Workspace {
    pub(super) fn activity_execution_active(&self) -> bool {
        self.activities
            .execution
            .live
            .as_ref()
            .is_some_and(|r| !r.report().status.terminal())
    }
    pub(super) fn activity_execution_automatic(&self) -> bool {
        self.activities.execution.automatic
    }
    pub(super) fn activity_execution_tick(&mut self) {
        let panel = &mut self.activities.execution;
        if let Some(live) = &mut panel.live {
            if !live.report().status.terminal() && !live.is_current(&self.session.editor) {
                live.step(&self.session.editor);
            }
            if panel.automatic && !live.report().status.terminal() {
                if panel
                    .next_tick
                    .is_none_or(|deadline| Instant::now() >= deadline)
                {
                    live.step(&self.session.editor);
                    panel.next_tick = Some(Instant::now() + Duration::from_millis(150));
                }
                self.context
                    .request_repaint_after(Duration::from_millis(150));
            }
            if live.report().status.terminal() {
                panel.automatic = false;
                panel.next_tick = None;
            }
        } else {
            panel.automatic = false;
            panel.next_tick = None;
        }
    }
    pub(super) fn activity_execution_ui(&mut self, ui: &mut egui::Ui, ready: bool) {
        let activity = self.activities.activity;
        let active = self.activity_execution_active();
        let automatic = self.activity_execution_automatic();
        let header=ui.collapsing("Finite execution controls",|ui|{
            ui.label("Single control token: literal true, otherwise Else. Actions are visited without running code. Pins/object values and forks are unsupported. Execution never sets verification Passed.");
            ui.label(activity.map_or("Select an accepted activity to prepare a run.".into(),|id|format!("Selected activity #{}",id.0)));
            ui.horizontal(|ui|{
                ui.label("Visit budget (1–4096)");let r=ui.add_enabled(!active,egui::TextEdit::singleline(&mut self.activities.execution.budget).char_limit(4).desired_width(100.));self.bound("activity:field:execution-budget",&r);
                let r=ui.add_enabled(ready&&!active&&activity.is_some(),egui::Button::new("Prepare fresh execution"));self.bound("activity:execution:Prepare",&r);
                if r.clicked(){
                    let result=self.activities.execution.budget.parse::<usize>().map_err(|_|"Enter a whole visit budget from 1 to 4096.".into()).and_then(|budget|Execution::start(&self.session.editor,activity.unwrap(),budget));
                    match result{Ok(live)=>{self.activities.execution.live=Some(live);self.activities.execution.note="Fresh execution prepared. Choose Step or Run.".into();},Err(error)=>{self.activities.execution.note=format!("Execution refused: {error}. Existing results and the model are retained.");}}
                }
            });
            ui.horizontal(|ui|{
                let r=ui.add_enabled(ready&&active&&!automatic,egui::Button::new("Step once"));self.bound("activity:execution:Step",&r);if r.clicked(){self.activities.execution.live.as_mut().unwrap().step(&self.session.editor);}
                let r=ui.add_enabled(ready&&active&&!automatic,egui::Button::new("Run bounded trace"));self.bound("activity:execution:Run",&r);if r.clicked(){self.activities.execution.automatic=true;self.activities.execution.next_tick=Some(Instant::now()+Duration::from_millis(150));self.context.request_repaint_after(Duration::from_millis(150));}
                let r=ui.add_enabled(active,egui::Button::new("Cancel execution"));self.bound("activity:execution:Cancel",&r);if r.clicked(){self.activities.execution.live.as_mut().unwrap().cancel(&self.session.editor);self.activities.execution.automatic=false;self.activities.execution.next_tick=None;}
                let r=ui.add_enabled(!active&&self.activities.execution.live.is_some(),egui::Button::new("Clear result"));self.bound("activity:execution:Clear",&r);if r.clicked(){self.activities.execution.live=None;self.activities.execution.note="Session result explicitly cleared; native model unchanged.".into();}
            });
            ui.label(&self.activities.execution.note);
            if let Some((current,report))=self.activities.execution.live.as_ref().map(|live|(live.is_current(&self.session.editor),live.report().clone())){
                let status=match &report.status{Status::Ready=>"Prepared".into(),Status::Running=>"Running".into(),Status::Completed=>"Completed".into(),Status::Deadlock{node}=>format!("Deadlock at node #{}",node.0),Status::BudgetExhausted=>"Step budget exhausted".into(),Status::Cancelled=>"Cancelled".into(),Status::StaleSource=>"Stopped: source changed".into()};
                ui.heading(format!("{status} · {} / {} visits",report.visits.len(),report.step_budget));
                ui.colored_label(if current{ACCENT}else{ERROR},if current{"Exact prepared source remains current."}else{"STALE / historical source. This result does not describe the current model."});
                ui.label(format!("Activity #{} · document {} · model revision {} · raw version {}",report.activity.0,report.source.document_id.0,report.source.model_revision,report.source.raw_version));
                ui.small(format!("{} · source SHA-256 {}",report.engine,report.snapshot_sha256));
                let r=ui.button("Copy historical report JSON");self.bound("activity:execution:Copy",&r);if r.clicked(){ui.ctx().copy_text(serde_json::to_string_pretty(&report).expect("finite report serializes"));}
                ui.label("Session result only. Save/reopen preserves the model, not a live execution; copied reports cannot resume or establish verification.");
                egui::ScrollArea::vertical().id_salt("finite-activity-trace").max_height(180.).show(ui,|ui|{for(i,visit)in report.visits.iter().enumerate(){ui.label(format!("{}: node #{}{}",i+1,visit.node.0,visit.flow.map_or(" · no outgoing flow".into(),|id|format!(" → flow #{}",id.0))));}});
            }
        });
        self.bound("activity:Execution controls", &header.header_response);
    }
    pub(super) fn activity_execution_audit(&self) -> serde_json::Value {
        let p = &self.activities.execution;
        serde_json::json!({"automatic":p.automatic,"budget":p.budget,"note":p.note,"report":p.live.as_ref().map(Execution::report),"current":p.live.as_ref().map(|r|r.is_current(&self.session.editor))})
    }
}

//! Native explicit controls over the one accepted flat state execution authority.
use super::*;
use mbse_model::types::{
    Id,
    state_machines::execution::{self as x, Execution, Status},
};
pub(super) struct Panel {
    pub live: Option<Execution>,
    pub automatic: bool,
    pub next_tick: Option<Instant>,
    pub budget: String,
    pub events: String,
    pub note: String,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            live: None,
            automatic: false,
            next_tick: None,
            budget: "128".into(),
            events: String::new(),
            note: String::new(),
        }
    }
}
fn event_ids(text: &str) -> Result<Vec<Id>, String> {
    if text.len() > x::MAX_EVENTS * 21 {
        return Err("Event input exceeds the finite4096-event limit.".into());
    }
    let mut ids = Vec::new();
    for token in text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        if ids.len() == x::MAX_EVENTS {
            return Err("Use at most4096 event IDs.".into());
        }
        let id = token
            .parse::<u64>()
            .ok()
            .filter(|id| *id > 0 && id.to_string() == token)
            .ok_or("Enter exact positive decimal Event IDs separated by commas or spaces.")?;
        ids.push(Id(id));
    }
    Ok(ids)
}
impl Workspace {
    pub(super) fn sm_execution_active(&self) -> bool {
        self.state_machines
            .execution
            .live
            .as_ref()
            .is_some_and(|r| !r.report().status.terminal())
    }
    pub(super) fn sm_execution_automatic(&self) -> bool {
        self.state_machines.execution.automatic
    }
    pub(super) fn sm_execution_tick(&mut self) {
        let p = &mut self.state_machines.execution;
        if let Some(live) = &mut p.live {
            if !live.report().status.terminal() && !live.is_current(&self.session.editor) {
                live.step(&self.session.editor);
            }
            if p.automatic && !live.report().status.terminal() {
                if p.next_tick
                    .is_none_or(|deadline| Instant::now() >= deadline)
                {
                    live.step(&self.session.editor);
                    p.next_tick = Some(Instant::now() + Duration::from_millis(150));
                }
                self.context
                    .request_repaint_after(Duration::from_millis(150));
            }
            if live.report().status.terminal() {
                p.automatic = false;
                p.next_tick = None;
            }
        } else {
            p.automatic = false;
            p.next_tick = None;
        }
    }
    pub(super) fn sm_execution_ui(&mut self, ui: &mut egui::Ui, ready: bool) {
        let machine = self.state_machines.machine;
        let active = self.sm_execution_active();
        let automatic = self.sm_execution_automatic();
        let header=ui.collapsing("Finite state execution controls",|ui|{
            ui.label("Completion transitions run first. True/unconditional are equal; Else is fallback. Conflicting routes stop without a choice. Unhandled owned events are consumed and recorded. Flat states only; no action code, payloads or verification result.");
            ui.label(machine.map_or("Select an accepted machine to prepare.".into(),|id|format!("Selected machine #{}",id.0)));
            if let Some(m)=mbse_model::types::load(&self.session.editor.snapshot).ok().and_then(|s|machine.and_then(|id|s.state_machines.get(&id).cloned())){ui.label(format!("Owned events: {}",m.events.values().map(|e|format!("{} #{}",e.name,e.id.0)).collect::<Vec<_>>().join("; ")));}
            ui.horizontal(|ui|{ui.label("Step budget (1–4096)");let r=ui.add_enabled(!active,egui::TextEdit::singleline(&mut self.state_machines.execution.budget).hint_text("128").char_limit(4).desired_width(100.));self.bound("state-machine:field:execution-budget",&r);});
            ui.horizontal(|ui|{ui.label("Finite event IDs, in order");let r=ui.add_enabled(!active,egui::TextEdit::singleline(&mut self.state_machines.execution.events).hint_text("7005, 7006 · empty stops when waiting for an event").char_limit(x::MAX_EVENTS*21).desired_width(560.));self.bound("state-machine:field:execution-events",&r);});
            let r=ui.add_enabled(ready&&!active&&machine.is_some(),egui::Button::new("Prepare fresh state execution"));self.bound("state-machine:execution:Prepare",&r);
            if r.clicked(){let p=&self.state_machines.execution;let result=p.budget.parse::<usize>().map_err(|_|"Enter a whole step budget from1to4096.".into()).and_then(|b|event_ids(&p.events).and_then(|events|Execution::start(&self.session.editor,machine.unwrap(),b,events)));match result{Ok(live)=>{self.state_machines.execution.live=Some(live);self.state_machines.execution.note="Fresh execution prepared; source and diagram retained.".into();}Err(error)=>self.state_machines.execution.note=format!("{error} Previous result and model retained.")}}
            ui.horizontal_wrapped(|ui|{
                let r=ui.add_enabled(ready&&active&&!automatic,egui::Button::new("Step once"));self.bound("state-machine:execution:Step",&r);if r.clicked(){self.state_machines.execution.live.as_mut().unwrap().step(&self.session.editor);}
                let r=ui.add_enabled(ready&&active&&!automatic,egui::Button::new("Run bounded state trace"));self.bound("state-machine:execution:Run",&r);if r.clicked(){self.state_machines.execution.automatic=true;self.state_machines.execution.next_tick=Some(Instant::now()+Duration::from_millis(150));self.context.request_repaint_after(Duration::from_millis(150));}
                let r=ui.add_enabled(active,egui::Button::new("Cancel execution"));self.bound("state-machine:execution:Cancel",&r);if r.clicked(){self.state_machines.execution.live.as_mut().unwrap().cancel(&self.session.editor);self.state_machines.execution.automatic=false;self.state_machines.execution.next_tick=None;}
                let r=ui.add_enabled(!active&&self.state_machines.execution.live.is_some(),egui::Button::new("Clear result"));self.bound("state-machine:execution:Clear",&r);if r.clicked(){self.state_machines.execution.live=None;self.state_machines.execution.note="Session result explicitly cleared; model retained.".into();}
            });ui.label(&self.state_machines.execution.note);
            if let Some((current,report))=self.state_machines.execution.live.as_ref().map(|r|(r.is_current(&self.session.editor),r.report().clone())){
                let status=match &report.status{Status::Ready=>"Prepared".into(),Status::Running=>"Running".into(),Status::Completed=>"Completed".into(),Status::WaitingForEvent{state}=>format!("Waiting for an event at State #{} · finite sequence ended",state.0),Status::Conflict{state,event,transitions}=>format!("Conflict at State #{} · {} · transitions {} · no route chosen",state.0,event.map_or("completion".into(),|id|format!("Event #{}",id.0)),transitions.iter().map(|id|format!("#{}",id.0)).collect::<Vec<_>>().join(", ")),Status::BudgetExhausted=>"Step budget exhausted".into(),Status::Cancelled=>"Cancelled".into(),Status::StaleSource=>"Stopped: source changed".into()};
                ui.heading(format!("{status} · {} / {} steps",report.steps.len(),report.step_budget));ui.label(format!("Current State #{} · consumed {} / {} queued events",report.current_state.0,report.consumed_events,report.events.len()));
                ui.colored_label(if current{ACCENT}else{ERROR},if current{"Exact prepared source remains current."}else{"STALE / historical source. This result does not describe the current model."});
                ui.label(format!("Machine #{} · document {} · model revision {} · raw version {}",report.machine.0,report.source.document_id.0,report.source.model_revision,report.source.raw_version));
                let r=ui.button("Copy historical report JSON");self.bound("state-machine:execution:Copy",&r);if r.clicked(){ui.ctx().copy_text(serde_json::to_string_pretty(&report).expect("finite report serializes"));}
                ui.label("Session result only. Save/reopen preserves model data, not a live execution; copied reports cannot resume or establish verification.");
                egui::ScrollArea::vertical().id_salt("finite-state-trace").max_height(180.).show(ui,|ui|{for(i,step)in report.steps.iter().enumerate(){ui.label(format!("{}: State #{} → #{} · {} · {}",i+1,step.source.0,step.target.0,step.event.map_or("completion".into(),|id|format!("Event #{}",id.0)),step.transition.map_or("unhandled event consumed".into(),|id|format!("Transition #{}",id.0))));}});
            }
        });
        self.bound("state-machine:Execution controls", &header.header_response);
    }
    pub(super) fn sm_execution_audit(&self) -> serde_json::Value {
        let p = &self.state_machines.execution;
        serde_json::json!({"automatic":p.automatic,"budget":p.budget,"events":p.events,"note":p.note,"report":p.live.as_ref().map(Execution::report),"current":p.live.as_ref().map(|r|r.is_current(&self.session.editor))})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_state_event_input_is_canonical_bounded_and_keeps_exact_order_and_duplicate_events() {
        assert_eq!(
            event_ids("1005, 1006\n1005").unwrap(),
            vec![Id(1005), Id(1006), Id(1005)]
        );
        assert!(event_ids("").unwrap().is_empty());
        for raw in ["0", "01", "+1", "-1", "1.0", "18446744073709551616", "one"] {
            assert!(event_ids(raw).is_err());
        }
        let exact = vec!["1005"; 4096].join(",");
        assert_eq!(event_ids(&exact).unwrap().len(), 4096);
        assert!(event_ids(&(exact + ",1005")).is_err());
        assert!(event_ids(&" ".repeat(x::MAX_EVENTS * 21 + 1)).is_err());
    }
}

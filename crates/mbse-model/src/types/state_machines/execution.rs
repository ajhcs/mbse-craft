//! Opt-in finite flat-state traversal. No event payload, action code or model writes.
use super::*;
use crate::stage1::{Editor, Freshness};
pub const ENGINE: &str = "mbse-craft.state-execution/1";
pub const POLICY: &str = "completion-first;true-or-unconditional;else-fallback;conflict-refuses;unhandled-event-consumed/1";
pub const MAX_STEPS: usize = 4096;
pub const MAX_EVENTS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Status {
    Ready,
    Running,
    Completed,
    WaitingForEvent {
        state: Id,
    },
    Conflict {
        state: Id,
        event: Option<Id>,
        transitions: Vec<Id>,
    },
    BudgetExhausted,
    Cancelled,
    StaleSource,
}
impl Status {
    pub fn terminal(&self) -> bool {
        !matches!(self, Self::Ready | Self::Running)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub source: Id,
    pub target: Id,
    pub transition: Option<Id>,
    pub event: Option<Id>,
}
/// Historical report only; it cannot authorize or reconstruct a live execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub engine: String,
    pub policy: String,
    pub source: Freshness,
    pub snapshot_sha256: String,
    pub machine: Id,
    pub step_budget: usize,
    pub events: Vec<Id>,
    pub consumed_events: usize,
    pub current_state: Id,
    pub status: Status,
    pub steps: Vec<Step>,
}
/// Process-local authority over an immutable accepted machine, never the Editor.
pub struct Execution {
    machine: Machine,
    report: Report,
}
impl Execution {
    pub fn start(
        editor: &Editor,
        machine: Id,
        step_budget: usize,
        events: Vec<Id>,
    ) -> Result<Self, String> {
        if !(1..=MAX_STEPS).contains(&step_budget) {
            return Err("STATE_EXECUTION_STEP_BUDGET_1_TO_4096".into());
        }
        if events.len() > MAX_EVENTS {
            return Err("STATE_EXECUTION_EVENT_LIMIT_4096".into());
        }
        editor.snapshot.validate()?;
        let store = super::super::load(&editor.snapshot)?;
        let machine = store
            .state_machines
            .get(&machine)
            .ok_or("STATE_EXECUTION_MACHINE_MISSING")?;
        if events.iter().any(|id| !machine.events.contains_key(id)) {
            return Err("STATE_EXECUTION_EVENT_NOT_OWNED".into());
        }
        let initial: Vec<_> = machine
            .states
            .values()
            .filter(|s| s.kind == StateKind::Initial)
            .collect();
        if initial.len() != 1 {
            return Err("STATE_EXECUTION_REQUIRES_ONE_INITIAL".into());
        }
        Ok(Self {
            machine: machine.clone(),
            report: Report {
                engine: ENGINE.into(),
                policy: POLICY.into(),
                source: editor.freshness(),
                snapshot_sha256: editor.snapshot.digest(),
                machine: machine.id,
                step_budget,
                events,
                consumed_events: 0,
                current_state: initial[0].id,
                status: Status::Ready,
                steps: Vec::new(),
            },
        })
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn is_current(&self, editor: &Editor) -> bool {
        self.report.source == editor.freshness()
            && self.report.snapshot_sha256 == editor.snapshot.digest()
    }
    fn candidates(&self, event: Option<Id>) -> Vec<Id> {
        let matching: Vec<_> = self
            .machine
            .transitions
            .values()
            .filter(|t| t.source == self.report.current_state && t.event == event)
            .collect();
        let enabled: Vec<_> = matching
            .iter()
            .filter(|t| matches!(t.guard, None | Some(Guard::Boolean(true))))
            .map(|t| t.id)
            .collect();
        if enabled.is_empty() {
            matching
                .iter()
                .filter(|t| t.guard == Some(Guard::Else))
                .map(|t| t.id)
                .collect()
        } else {
            enabled
        }
    }
    /// One transition or one consumed unhandled event. Conflict never consumes.
    pub fn step(&mut self, editor: &Editor) -> &Report {
        if self.report.status.terminal() {
            return &self.report;
        }
        if !self.is_current(editor) {
            self.report.status = Status::StaleSource;
            return &self.report;
        }
        if self.report.steps.len() >= self.report.step_budget {
            self.report.status = Status::BudgetExhausted;
            return &self.report;
        }
        let mut event = None;
        let mut candidates = self.candidates(None);
        if candidates.is_empty() {
            let Some(next) = self.report.events.get(self.report.consumed_events).copied() else {
                self.report.status = Status::WaitingForEvent {
                    state: self.report.current_state,
                };
                return &self.report;
            };
            event = Some(next);
            candidates = self.candidates(event);
        }
        if candidates.len() > 1 {
            self.report.status = Status::Conflict {
                state: self.report.current_state,
                event,
                transitions: candidates,
            };
            return &self.report;
        }
        let source = self.report.current_state;
        let transition = candidates.first().copied();
        let target = transition.map_or(source, |id| self.machine.transitions[&id].target);
        if event.is_some() {
            self.report.consumed_events += 1;
        }
        self.report.steps.push(Step {
            source,
            target,
            transition,
            event,
        });
        self.report.current_state = target;
        self.report.status = if self.machine.states[&target].kind == StateKind::Final {
            Status::Completed
        } else {
            Status::Running
        };
        &self.report
    }
    pub fn cancel(&mut self, editor: &Editor) -> &Report {
        if !self.report.status.terminal() {
            self.report.status = if self.is_current(editor) {
                Status::Cancelled
            } else {
                Status::StaleSource
            };
        }
        &self.report
    }
}

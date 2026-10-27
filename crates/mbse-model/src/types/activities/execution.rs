//! Opt-in finite, single-control-token traversal of the existing accepted activity.
//! Actions are visited; no action code, object values, scripts, solver or I/O execute.
use super::*;
use crate::stage1::{Editor, Freshness};
pub const ENGINE: &str = "mbse-craft.activity-control/1";
pub const MAX_STEPS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Status {
    Ready,
    Running,
    Completed,
    Deadlock { node: Id },
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
pub struct Visit {
    pub node: Id,
    /// Exact accepted control flow chosen after visiting this node, if any.
    pub flow: Option<Id>,
}
/// Serializable historical data only. It cannot reconstruct a live Execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub engine: String,
    pub source: Freshness,
    pub snapshot_sha256: String,
    pub activity: Id,
    pub step_budget: usize,
    pub status: Status,
    pub visits: Vec<Visit>,
}
/// Live authority is process-local and not deserializable. Never mutates the Editor.
pub struct Execution {
    activity: Activity,
    next: Id,
    report: Report,
}
impl Execution {
    pub fn start(editor: &Editor, activity: Id, step_budget: usize) -> Result<Self, String> {
        if !(1..=MAX_STEPS).contains(&step_budget) {
            return Err("ACTIVITY_EXECUTION_STEP_BUDGET_1_TO_4096".into());
        }
        editor.snapshot.validate()?;
        let store = super::super::load(&editor.snapshot)?;
        let a = store
            .activities
            .get(&activity)
            .ok_or("ACTIVITY_EXECUTION_ACTIVITY_MISSING")?;
        // These accepted authoring constructs have no value/token semantics here.
        if !a.pins.is_empty() || a.flows.values().any(|f| f.kind == FlowKind::Object) {
            return Err("ACTIVITY_EXECUTION_OBJECT_PINS_OR_FLOWS_UNSUPPORTED".into());
        }
        let initial: Vec<_> = a
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Initial)
            .collect();
        if initial.len() != 1 {
            return Err("ACTIVITY_EXECUTION_REQUIRES_ONE_INITIAL".into());
        }
        for n in a.nodes.values() {
            let outgoing = a.flows.values().filter(|f| f.source == n.id).count();
            if n.kind != NodeKind::Decision && outgoing > 1 {
                return Err("ACTIVITY_EXECUTION_FORK_UNSUPPORTED".into());
            }
        }
        Ok(Self {
            activity: a.clone(),
            next: initial[0].id,
            report: Report {
                engine: ENGINE.into(),
                source: editor.freshness(),
                snapshot_sha256: editor.snapshot.digest(),
                activity,
                step_budget,
                status: Status::Ready,
                visits: Vec::new(),
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
    /// One bounded visit. Terminal calls are idempotent; stale historical reports
    /// remain historical and must be labelled using is_current, never re-authorized.
    pub fn step(&mut self, editor: &Editor) -> &Report {
        if self.report.status.terminal() {
            return &self.report;
        }
        if !self.is_current(editor) {
            self.report.status = Status::StaleSource;
            return &self.report;
        }
        if self.report.visits.len() >= self.report.step_budget {
            self.report.status = Status::BudgetExhausted;
            return &self.report;
        }
        let node = &self.activity.nodes[&self.next];
        if node.kind == NodeKind::Final {
            self.report.visits.push(Visit {
                node: node.id,
                flow: None,
            });
            self.report.status = Status::Completed;
            return &self.report;
        }
        let flows: Vec<_> = self
            .activity
            .flows
            .values()
            .filter(|f| f.source == node.id)
            .collect();
        // Only a literal true branch is enabled. Else is used only when no true
        // exists; false is disabled. Accepted models enforce unique guard kinds.
        let chosen = if node.kind == NodeKind::Decision {
            flows
                .iter()
                .find(|f| f.guard == Some(Guard::Boolean(true)))
                .copied()
                .or_else(|| flows.iter().find(|f| f.guard == Some(Guard::Else)).copied())
        } else {
            flows.first().copied()
        };
        self.report.visits.push(Visit {
            node: node.id,
            flow: chosen.map(|f| f.id),
        });
        if let Some(flow) = chosen {
            self.next = flow.target;
            self.report.status = Status::Running;
        } else {
            self.report.status = Status::Deadlock { node: node.id };
        }
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

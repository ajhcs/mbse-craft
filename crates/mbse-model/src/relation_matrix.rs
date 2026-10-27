//! Bounded matrix projection and sealed edit requests over existing native graph commands.
use crate::{
    stage1::{Command, Editor, RecordId, RecordKind, SubjectRef},
    stage1_calc::DocumentStamp,
    stage1_graph::{self, RelationRole},
};
use serde::{Deserialize, Serialize};
pub const PAGE: usize = 32;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    ContributesTo,
    Subject,
    ForObjective,
    Scope,
    CalculationInput,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ContributesTo => "Capability contributes to Mission",
            Self::Subject => "Objective subject",
            Self::ForObjective => "Measure for Objective",
            Self::Scope => "System scope (read-only)",
            Self::CalculationInput => "Calculation dependency (read-only)",
        }
    }
    fn role(self) -> RelationRole {
        match self {
            Self::ContributesTo => RelationRole::ContributesTo,
            Self::Subject => RelationRole::Subject,
            Self::ForObjective => RelationRole::ForObjective,
            Self::Scope => RelationRole::Scope,
            Self::CalculationInput => RelationRole::CalculationInput,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub kind: Kind,
    pub row_offset: usize,
    pub column_offset: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct Axis {
    pub id: RecordId,
    pub kind: RecordKind,
    pub label: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Cell {
    pub source: RecordId,
    pub target: RecordId,
    pub present: bool,
    pub editable: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    guard: DocumentStamp,
    query: Query,
    total: usize,
    rows: Vec<Axis>,
    columns: Vec<Axis>,
    cells: Vec<Cell>,
}
#[derive(Clone, Debug)]
pub struct Edit {
    guard: DocumentStamp,
    command: Command,
}
impl Edit {
    pub fn resolve(&self, e: &Editor) -> Result<Command, String> {
        if self.guard != e.calculation_stamp() {
            return Err("MATRIX_SOURCE_STALE".into());
        }
        Ok(self.command.clone())
    }
}
fn command(
    e: &Editor,
    kind: Kind,
    source: RecordId,
    target: RecordId,
    present: bool,
) -> Result<Command, String> {
    let a = e
        .snapshot
        .records
        .get(&source)
        .ok_or("MATRIX_SOURCE_MISSING")?;
    let b = e
        .snapshot
        .records
        .get(&target)
        .ok_or("MATRIX_TARGET_MISSING")?;
    if matches!(kind, Kind::Scope | Kind::CalculationInput) {
        return Err("Read-only here; edit scope or calculation bindings through their existing authoring workflow.".into());
    }
    if a.scope != b.scope {
        return Err("Cross-System relation unsupported by the existing command.".into());
    }
    match kind {
        Kind::ContributesTo
            if a.kind == RecordKind::Capability && b.kind == RecordKind::Mission =>
        {
            Ok(Command::ToggleMission {
                capability: source,
                mission: target,
            })
        }
        Kind::Subject if a.kind == RecordKind::Objective => {
            let subject = match b.kind {
                RecordKind::System => SubjectRef::System(target),
                RecordKind::Mission => SubjectRef::Mission(target),
                RecordKind::Capability => SubjectRef::Capability(target),
                _ => return Err("Objective subject must be System, Mission or Capability.".into()),
            };
            Ok(Command::SetSubject {
                objective: source,
                subject: (!present).then_some(subject),
            })
        }
        Kind::ForObjective if a.kind == RecordKind::Measure && b.kind == RecordKind::Objective => {
            if present {
                Err(
                    "Measure requires an Objective; select another Objective to replace this link."
                        .into(),
                )
            } else {
                Ok(Command::SetForObjective {
                    measure: source,
                    objective: target,
                })
            }
        }
        _ => Err("Wrong endpoint kinds for this relation command.".into()),
    }
}
impl Report {
    pub fn query(&self) -> Query {
        self.query
    }
    pub fn total(&self) -> usize {
        self.total
    }
    pub fn rows(&self) -> &[Axis] {
        &self.rows
    }
    pub fn columns(&self) -> &[Axis] {
        &self.columns
    }
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
    pub fn check(&self, e: &Editor) -> Result<(), String> {
        if self.guard != e.calculation_stamp() {
            Err("MATRIX_SOURCE_STALE".into())
        } else {
            Ok(())
        }
    }
    pub fn edit(&self, e: &Editor, source: RecordId, target: RecordId) -> Result<Edit, String> {
        self.check(e)?;
        let cell = self
            .cells
            .iter()
            .find(|c| c.source == source && c.target == target)
            .ok_or("MATRIX_CELL_NOT_ON_PAGE")?;
        if !cell.editable {
            return Err(cell
                .reason
                .clone()
                .unwrap_or_else(|| "MATRIX_CELL_READ_ONLY".into()));
        }
        let cmd = command(e, self.query.kind, source, target, cell.present)?;
        // Eligibility is only a hint. Existing command authority must accept the exact proposal.
        let mut preview = e.clone();
        if !preview.command(&preview.freshness(), cmd.clone())?.accepted {
            return Err("MATRIX_COMMAND_REFUSED".into());
        }
        Ok(Edit {
            guard: self.guard.clone(),
            command: cmd,
        })
    }
    pub fn axis(&self, e: &Editor, id: RecordId) -> Result<RecordId, String> {
        self.check(e)?;
        if self.rows.iter().chain(&self.columns).any(|a| a.id == id) {
            Ok(id)
        } else {
            Err("MATRIX_AXIS_NOT_ON_PAGE".into())
        }
    }
}
pub fn query(e: &Editor, mut q: Query) -> Result<Report, String> {
    let graph = stage1_graph::derive_graph(&e.snapshot)?;
    let present_edges = graph
        .edges
        .iter()
        .filter(|x| x.id.role == q.kind.role())
        .map(|x| (x.id.source, x.id.target))
        .collect::<std::collections::BTreeSet<_>>();
    let records: Vec<_> = e.snapshot.records.values().collect();
    let total = records.len();
    q.row_offset = q.row_offset.min(total);
    q.column_offset = q.column_offset.min(total);
    let axes = |offset: usize| {
        records
            .iter()
            .skip(offset)
            .take(PAGE)
            .map(|r| {
                let mut label: String = r.name.chars().take(64).collect();
                if r.name.chars().nth(64).is_some() {
                    label.push_str(" [shortened]");
                }
                Axis {
                    id: r.id,
                    kind: r.kind,
                    label,
                }
            })
            .collect::<Vec<_>>()
    };
    let rows = axes(q.row_offset);
    let columns = axes(q.column_offset);
    let mut cells = vec![];
    for a in &rows {
        for b in &columns {
            let present = present_edges.contains(&(a.id, b.id));
            let reason = command(e, q.kind, a.id, b.id, present).err();
            cells.push(Cell {
                source: a.id,
                target: b.id,
                present,
                editable: reason.is_none(),
                reason,
            });
        }
    }
    Ok(Report {
        guard: e.calculation_stamp(),
        query: q,
        total,
        rows,
        columns,
        cells,
    })
}

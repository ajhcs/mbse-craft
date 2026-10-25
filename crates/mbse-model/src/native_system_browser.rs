//! Read-only System membership, not inferred decomposition or ownership.
use crate::stage1::{Record, RecordId, RecordKind, Snapshot};
use std::collections::BTreeMap;
pub const LABEL_SCALARS: usize = 64;
#[derive(Debug, PartialEq, Eq)]
pub struct Label {
    pub id: RecordId,
    pub kind: RecordKind,
    pub name: String,
    pub shortened: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    pub system: Label,
    pub members: Vec<Label>,
}
fn label(record: &Record) -> Label {
    Label {
        id: record.id,
        kind: record.kind,
        name: if record.name.is_empty() {
            "Unnamed".into()
        } else {
            record.name.chars().take(LABEL_SCALARS).collect()
        },
        shortened: record.name.chars().nth(LABEL_SCALARS).is_some(),
    }
}
pub fn project(snapshot: &Snapshot) -> Result<Vec<Group>, String> {
    snapshot.validate()?;
    if snapshot
        .records
        .iter()
        .any(|(key, record)| *key != record.id)
    {
        return Err("NATIVE_BROWSER_RECORD_KEY_MISMATCH".into());
    }
    let mut groups: BTreeMap<_, _> = snapshot
        .records
        .values()
        .filter(|r| r.kind == RecordKind::System)
        .map(|r| {
            (
                r.id,
                Group {
                    system: label(r),
                    members: Vec::new(),
                },
            )
        })
        .collect();
    for record in snapshot
        .records
        .values()
        .filter(|r| r.kind != RecordKind::System)
    {
        groups
            .get_mut(&record.scope)
            .ok_or("NATIVE_BROWSER_SYSTEM_SCOPE")?
            .members
            .push(label(record));
    }
    Ok(groups.into_values().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::{DocumentId, Editor};
    fn fixture() -> Snapshot {
        let mut s = Snapshot::from_fixture(
            DocumentId("system-browser-test".into()),
            &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
        )
        .unwrap();
        s.records.insert(
            RecordId(2),
            Record {
                id: RecordId(2),
                kind: RecordKind::System,
                name: "Second System".into(),
                scope: RecordId(2),
                fields: BTreeMap::from([("purpose".into(), serde_json::json!("Synthetic test"))]),
            },
        );
        s.records.get_mut(&RecordId(100)).unwrap().name = "Duplicate".into();
        let mut member = s.records[&RecordId(100)].clone();
        member.id = RecordId(2000);
        member.scope = RecordId(2);
        s.records.insert(member.id, member);
        s.next_id = 3000;
        s
    }
    #[test]
    fn exact_membership_keeps_duplicate_names_and_typed_relations_distinct() {
        let s = fixture();
        let g = project(&s).unwrap();
        assert_eq!(
            g.iter().map(|x| x.system.id).collect::<Vec<_>>(),
            vec![RecordId(1), RecordId(2)]
        );
        assert!(g[0]
            .members
            .iter()
            .any(|m| m.id == RecordId(100) && m.name == "Duplicate"));
        assert_eq!(g[1].members[0].id, RecordId(2000));
        assert_eq!(g[1].members[0].name, "Duplicate");
        assert!(g[0].members.iter().any(|m| m.id == RecordId(301))); // Measure belongs to System scope, not an invented Objective group
        let all: Vec<_> = g
            .iter()
            .flat_map(|g| std::iter::once(g.system.id).chain(g.members.iter().map(|m| m.id)))
            .collect();
        let unique: std::collections::BTreeSet<_> = all.iter().copied().collect();
        assert_eq!(all.len(), s.records.len());
        assert_eq!(unique.len(), all.len());
        assert!(g
            .iter()
            .all(|x| x.members.windows(2).all(|w| w[0].id < w[1].id)));
    }
    #[test]
    fn group_and_member_labels_are_bounded_without_shortening_ids() {
        let mut s = fixture();
        s.records.get_mut(&RecordId(1)).unwrap().name = "界".repeat(100);
        let id = RecordId(9007199254741001);
        let mut x = s.records[&RecordId(100)].clone();
        x.id = id;
        x.name = "界".repeat(100);
        s.records.insert(id, x);
        s.next_id = id.0 + 1;
        let g = project(&s).unwrap();
        assert!(g[0].system.shortened);
        assert_eq!(g[0].system.name.chars().count(), LABEL_SCALARS);
        assert!(g[0].system.name.len() <= 4 * LABEL_SCALARS);
        let m = g[0].members.iter().find(|m| m.id == id).unwrap();
        assert!(m.shortened);
        assert_eq!(m.name.chars().count(), LABEL_SCALARS);
        assert_eq!(m.id, id);
    }
    #[test]
    fn malformed_scope_and_key_refuse_without_partial_projection_or_repair() {
        for scope in [RecordId(999999), RecordId(100)] {
            let mut s = fixture();
            s.records.get_mut(&RecordId(2000)).unwrap().scope = scope;
            let before = s.clone();
            assert!(project(&s).is_err());
            assert_eq!(s, before);
        }
        let mut s = fixture();
        s.records.get_mut(&RecordId(2)).unwrap().scope = RecordId(1);
        assert!(project(&s).is_err());
        let mut s = fixture();
        let m = s.records.remove(&RecordId(2000)).unwrap();
        s.records.insert(RecordId(2001), m);
        assert!(project(&s).is_err());
    }
    #[test]
    fn projection_is_deterministic_and_preserves_native_values_and_raw_state() {
        let s = fixture();
        let e = Editor::new(s, "browser-runtime".into()).unwrap();
        let before = (
            e.snapshot.digest(),
            e.freshness(),
            e.calculation_stamp(),
            e.selected,
        );
        assert_eq!(project(&e.snapshot).unwrap(), project(&e.snapshot).unwrap());
        assert_eq!(
            (
                e.snapshot.digest(),
                e.freshness(),
                e.calculation_stamp(),
                e.selected
            ),
            before
        );
    }
}

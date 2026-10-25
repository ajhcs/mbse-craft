//! Bounded read-only native record search. Names are labels, never identity.
use crate::{
    native_views::{Stamp, Views},
    stage1::{Editor, RecordId, RecordKind},
};
pub const MAX_HITS: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    NameContains,
    ExactId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub mode: Mode,
    pub text: String,
    pub kind: Option<RecordKind>,
    pub system: Option<RecordId>,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            mode: Mode::NameContains,
            text: String::new(),
            kind: None,
            system: None,
        }
    }
}
#[derive(Debug)]
pub struct Hit {
    pub record: RecordId,
    pub kind: RecordKind,
    pub system: RecordId,
    pub name_preview: String,
    pub name_truncated: bool,
}
pub struct Report {
    stamp: Stamp,
    query: Query,
    hits: Vec<Hit>,
    total: usize,
}
impl Report {
    pub fn query(&self) -> &Query {
        &self.query
    }
    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }
    pub fn total(&self) -> usize {
        self.total
    }
    pub fn check(&self, editor: &Editor, views: &Views) -> Result<(), String> {
        if views.stamp(editor) != self.stamp {
            return Err("NATIVE_SEARCH_STALE: run search again before navigating".into());
        }
        Ok(())
    }
    pub fn record(&self, editor: &Editor, views: &Views, index: usize) -> Result<RecordId, String> {
        self.check(editor, views)?;
        let id = self.hits.get(index).ok_or("NATIVE_SEARCH_INDEX")?.record;
        if !editor.snapshot.records.contains_key(&id) {
            return Err("NATIVE_SEARCH_RECORD_MISSING".into());
        }
        Ok(id)
    }
}
pub fn search(editor: &Editor, views: &Views, query: Query) -> Result<Report, String> {
    if query.text.len() > 1024 {
        return Err("NATIVE_SEARCH_QUERY_BUDGET: use at most 1024 UTF-8 bytes".into());
    }
    editor.snapshot.validate()?;
    if let Some(scope) = query.system {
        if editor.snapshot.records.get(&scope).map(|r| r.kind) != Some(RecordKind::System) {
            return Err("NATIVE_SEARCH_SYSTEM_SCOPE".into());
        }
    }
    let exact = if query.mode == Mode::ExactId {
        let id = query
            .text
            .parse::<u64>()
            .map_err(|_| "NATIVE_SEARCH_EXACT_ID: use canonical positive decimal")?;
        if id == 0 || id.to_string() != query.text {
            return Err("NATIVE_SEARCH_EXACT_ID: use canonical positive decimal".into());
        }
        Some(RecordId(id))
    } else {
        None
    };
    let mut hits = Vec::new();
    let mut total = 0;
    for record in editor.snapshot.records.values() {
        if query.system.is_some_and(|s| record.scope != s)
            || query.kind.is_some_and(|k| record.kind != k)
        {
            continue;
        }
        let matched = match exact {
            Some(id) => record.id == id,
            None => record.name.contains(&query.text),
        };
        if matched {
            total += 1;
            if hits.len() < MAX_HITS {
                hits.push(Hit {
                    record: record.id,
                    kind: record.kind,
                    system: record.scope,
                    name_preview: record.name.chars().take(256).collect(),
                    name_truncated: record.name.chars().nth(256).is_some(),
                });
            }
        }
    }
    Ok(Report {
        stamp: views.stamp(editor),
        query,
        hits,
        total,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::{DocumentId, Record, Snapshot};
    use std::collections::BTreeMap;
    fn fixture() -> (Editor, Views) {
        let snapshot = Snapshot::from_fixture(
            DocumentId("native-search".into()),
            &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
        )
        .unwrap();
        let e = Editor::new(snapshot, "search-runtime".into()).unwrap();
        let v = Views::new(&e).unwrap();
        (e, v)
    }
    #[test]
    fn duplicate_labels_keep_exact_ids_kind_and_actual_system_scope() {
        let (mut e, v) = fixture();
        for id in [100, 200] {
            e.snapshot.records.get_mut(&RecordId(id)).unwrap().name = "Duplicate".into();
        }
        e.snapshot.records.insert(
            RecordId(2),
            Record {
                id: RecordId(2),
                kind: RecordKind::System,
                name: "Other".into(),
                scope: RecordId(2),
                fields: BTreeMap::from([("purpose".into(), serde_json::json!(""))]),
            },
        );
        let mut other = e.snapshot.records[&RecordId(100)].clone();
        other.id = RecordId(2000);
        other.scope = RecordId(2);
        e.snapshot.records.insert(other.id, other);
        e.snapshot.next_id = 3000;
        let q = Query {
            mode: Mode::NameContains,
            text: "Duplicate".into(),
            kind: None,
            system: Some(RecordId(1)),
        };
        let report = search(&e, &v, q.clone()).unwrap();
        assert_eq!(
            report.hits().iter().map(|h| h.record).collect::<Vec<_>>(),
            vec![RecordId(100), RecordId(200)]
        );
        let mut q = q;
        q.kind = Some(RecordKind::Mission);
        assert_eq!(
            search(&e, &v, q.clone()).unwrap().hits()[0].record,
            RecordId(100)
        );
        q.system = Some(RecordId(2));
        assert_eq!(search(&e, &v, q).unwrap().hits()[0].record, RecordId(2000));
        let q = Query {
            mode: Mode::NameContains,
            text: "duplicate".into(),
            kind: None,
            system: None,
        };
        assert_eq!(search(&e, &v, q).unwrap().total(), 0);
    }
    #[test]
    fn name_digits_and_canonical_full_width_id_are_separate_modes() {
        let (mut e, v) = fixture();
        e.snapshot.records.get_mut(&RecordId(100)).unwrap().name = "200".into();
        let q = Query {
            mode: Mode::NameContains,
            text: "200".into(),
            ..Default::default()
        };
        assert_eq!(search(&e, &v, q).unwrap().hits()[0].record, RecordId(100));
        for bad in [
            "0",
            "001",
            "+1",
            "1e3",
            "1.0",
            " 1",
            "#1",
            "18446744073709551616",
            "",
        ] {
            assert!(search(
                &e,
                &v,
                Query {
                    mode: Mode::ExactId,
                    text: bad.into(),
                    ..Default::default()
                }
            )
            .is_err());
        }
        let id = RecordId(9007199254741001);
        let mut high = e.snapshot.records[&RecordId(100)].clone();
        high.id = id;
        e.snapshot.records.insert(id, high);
        e.snapshot.next_id = id.0 + 1;
        let r = search(
            &e,
            &v,
            Query {
                mode: Mode::ExactId,
                text: id.0.to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.hits()[0].record, id);
    }
    #[test]
    fn result_and_label_prefix_budgets_are_explicit_and_count_all_matches() {
        let (mut e, v) = fixture();
        for id in 1000..1300 {
            let mut x = e.snapshot.records[&RecordId(100)].clone();
            x.id = RecordId(id);
            x.name = "界".repeat(300);
            e.snapshot.records.insert(x.id, x);
        }
        e.snapshot.next_id = 2000;
        let r = search(
            &e,
            &v,
            Query {
                kind: Some(RecordKind::Mission),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.total(), 301);
        assert_eq!(r.hits().len(), MAX_HITS);
        let h = r
            .hits()
            .iter()
            .find(|h| h.record == RecordId(1000))
            .unwrap();
        assert!(h.name_truncated);
        assert_eq!(h.name_preview.chars().count(), 256);
        assert_eq!(h.name_preview.len(), 768);
        assert!(search(
            &e,
            &v,
            Query {
                text: "x".repeat(1025),
                ..Default::default()
            }
        )
        .is_err());
    }
    #[test]
    fn unknown_or_wrong_kind_scope_refuses_and_empty_supported_query_is_distinct() {
        let (e, v) = fixture();
        for scope in [RecordId(999999), RecordId(100)] {
            assert!(search(
                &e,
                &v,
                Query {
                    system: Some(scope),
                    ..Default::default()
                }
            )
            .is_err());
        }
        let r = search(
            &e,
            &v,
            Query {
                text: "no such native record".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.total(), 0);
        assert!(r.hits().is_empty());
        assert!(r.record(&e, &v, 0).is_err());
    }
    #[test]
    fn sealed_navigation_stales_on_runtime_raw_layout_and_preserves_target_on_selection() {
        let (mut e, mut v) = fixture();
        let q = Query {
            mode: Mode::ExactId,
            text: "100".into(),
            ..Default::default()
        };
        let r = search(&e, &v, q.clone()).unwrap();
        let before = (
            e.snapshot.digest(),
            v.document().digest().unwrap(),
            v.history_usage(),
            v.document().next_id(),
        );
        let id = r.record(&e, &v, 0).unwrap();
        e.select(id).unwrap();
        assert_eq!(e.selected, Some(RecordId(100)));
        assert_eq!(
            (
                e.snapshot.digest(),
                v.document().digest().unwrap(),
                v.history_usage(),
                v.document().next_id()
            ),
            before
        );
        assert!(r.check(&e, &v).is_ok());
        let other = Views::new(&e).unwrap();
        assert!(r.record(&e, &other, 0).is_err());
        e.snapshot.raw_version += 1;
        assert!(r.record(&e, &v, 0).is_err());
        let r = search(&e, &v, q).unwrap();
        let stamp = v.stamp(&e);
        v.apply(
            &e,
            &stamp,
            crate::native_views::Layout::AddView("stales query".into()),
        )
        .unwrap();
        assert!(r.record(&e, &v, 0).is_err());
    }
}

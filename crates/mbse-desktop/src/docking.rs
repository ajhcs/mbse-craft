//! App-owned fixed panel registry over the pinned Craft transaction model.
use craft_ui::{
    docking::{Action, Layout, Node, Permissions},
    layout::{SplitAxis, SplitSize},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Panel {
    Browser,
    Inspector,
    Canvas,
}
impl Panel {
    pub const ALL: [Self; 3] = [Self::Browser, Self::Inspector, Self::Canvas];
    pub fn key(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::Inspector => "inspector",
            Self::Canvas => "canvas",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Browser => "Model browser",
            Self::Inspector => "Properties",
            Self::Canvas => "Native views",
        }
    }
    pub fn permissions(self) -> Permissions {
        Permissions {
            closable: false,
            ..Default::default()
        }
    }
}
pub struct WorkspaceDock {
    pub layout: Layout<Panel>,
    pub revision: u64,
    pub window: bool,
    pub note: String,
    pub last_actions: Vec<String>,
    pub body_ids: std::collections::BTreeMap<String, String>,
}
impl Default for WorkspaceDock {
    fn default() -> Self {
        let tabs = |panel| {
            Box::new(Node::Tabs {
                panels: vec![panel],
                active: 0,
            })
        };
        Self {
            layout: Layout {
                root: Some(Node::Split {
                    axis: SplitAxis::Horizontal,
                    size: SplitSize::FixedFirst(340.),
                    first: Box::new(Node::Split {
                        axis: SplitAxis::Vertical,
                        size: SplitSize::FixedFirst(400.),
                        first: tabs(Panel::Browser),
                        second: tabs(Panel::Inspector),
                    }),
                    second: tabs(Panel::Canvas),
                }),
                floating: vec![],
            },
            revision: 0,
            window: false,
            note: String::new(),
            last_actions: vec![],
            body_ids: Default::default(),
        }
    }
}
impl WorkspaceDock {
    pub fn check(layout: &Layout<Panel>) -> Result<(), String> {
        layout
            .validate()
            .map_err(|e| format!("DOCK_INVALID: {e:?}"))?;
        let mut panels: Vec<_> = layout.panels().into_iter().copied().collect();
        panels.sort();
        if panels != Panel::ALL {
            return Err("DOCK_REGISTRY: every owned panel must occur exactly once".into());
        }
        Ok(())
    }
    pub fn apply(&mut self, expected: u64, actions: Vec<Action<Panel>>) -> Result<(), String> {
        if expected != self.revision {
            return Err("DOCK_STALE: retry from the current panel layout".into());
        }
        Self::check(&self.layout)?;
        let mut next = self.layout.clone();
        for action in &actions {
            next.apply_with_permissions(action.clone(), |p| p.permissions())
                .map_err(|e| format!("DOCK_REFUSED: {e:?}"))?;
        }
        Self::check(&next)?;
        if next == self.layout {
            return Ok(());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("DOCK_REVISION_EXHAUSTED")?;
        self.last_actions = actions.iter().map(|a| format!("{a:?}")).collect();
        self.layout = next;
        self.revision = revision;
        self.note = "Panel layout updated. Native model, selection and raw drafts retained.".into();
        Ok(())
    }
    pub fn audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.workspace-docking/1","revision":self.revision,"panels":self.layout.panels().into_iter().map(|p|p.key()).collect::<Vec<_>>(),"layout":format!("{:?}",self.layout),"floating":self.layout.floating.iter().map(|f|serde_json::json!({"panels":f.panels.iter().map(|p|p.key()).collect::<Vec<_>>(),"active":f.active,"rect":f.rect})).collect::<Vec<_>>(),"note":self.note,"last_actions":self.last_actions,"body_ids":self.body_ids,"controls_open":self.window})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_ui::docking::{Placement, Zone};
    #[test]
    fn moving_tabbing_splitting_and_floating_keep_each_owned_identity_once() {
        let mut d = WorkspaceDock::default();
        WorkspaceDock::check(&d.layout).unwrap();
        for a in [
            Action::Float {
                panel: Panel::Inspector,
                rect: [20., 30., 400., 600.],
            },
            Action::Move {
                panel: Panel::Inspector,
                anchor: Panel::Browser,
                placement: Placement::Tab { before: None },
            },
            Action::Move {
                panel: Panel::Inspector,
                anchor: Panel::Canvas,
                placement: Placement::Split(Zone::Left),
            },
        ] {
            let rev = d.revision;
            d.apply(rev, vec![a]).unwrap();
            assert_eq!(d.revision, rev + 1);
            WorkspaceDock::check(&d.layout).unwrap();
            assert_eq!(d.layout.panels().len(), 3);
        }
    }
    #[test]
    fn invalid_batch_and_geometry_refuse_without_losing_source_or_revision() {
        let mut d = WorkspaceDock::default();
        let before = d.layout.clone();
        let actions = vec![
            Action::Float {
                panel: Panel::Inspector,
                rect: [20., 30., 400., 600.],
            },
            Action::Close {
                panel: Panel::Browser,
            },
        ];
        assert!(d.apply(0, actions).is_err());
        assert_eq!(d.layout, before);
        assert_eq!(d.revision, 0);
        assert!(d.last_actions.is_empty());
        for rect in [
            [f32::NAN, 0., 200., 200.],
            [0., 0., -20., 200.],
            [0., 0., f32::INFINITY, 200.],
        ] {
            assert!(d
                .apply(
                    0,
                    vec![Action::Float {
                        panel: Panel::Inspector,
                        rect
                    }]
                )
                .is_err());
            assert_eq!(d.layout, before);
            assert_eq!(d.revision, 0);
        }
    }
    #[test]
    fn stale_exhausted_and_incomplete_registry_refuse_atomically() {
        let mut d = WorkspaceDock::default();
        d.apply(
            0,
            vec![Action::Float {
                panel: Panel::Inspector,
                rect: [20., 30., 400., 600.],
            }],
        )
        .unwrap();
        let before = d.layout.clone();
        assert!(d
            .apply(
                0,
                vec![Action::Activate {
                    panel: Panel::Browser
                }]
            )
            .unwrap_err()
            .contains("STALE"));
        assert_eq!(d.layout, before);
        d.revision = u64::MAX;
        assert!(d
            .apply(
                u64::MAX,
                vec![Action::Float {
                    panel: Panel::Browser,
                    rect: [40., 40., 300., 300.]
                }]
            )
            .unwrap_err()
            .contains("EXHAUSTED"));
        assert_eq!(d.layout, before);
        let bad = Layout {
            root: Some(Node::Tabs {
                panels: vec![Panel::Canvas],
                active: 0,
            }),
            floating: vec![],
        };
        assert!(WorkspaceDock::check(&bad).unwrap_err().contains("REGISTRY"));
    }
}

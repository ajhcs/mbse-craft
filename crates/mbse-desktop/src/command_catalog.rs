//! App-owned workspace commands. IDs are durable API, not translated labels.
//! Default bindings; P10 resolves validated overrides by editing scope.
use eframe::egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Id {
    New,
    Open,
    Recover,
    Import,
    Save,
    Recovery,
    Export,
    Recoveries,
    Quit,
    Undo,
    Redo,
    LayoutUndo,
    LayoutRedo,
    Search,
    Usages,
    Impact,
    Presentation,
    External,
    Assurance,
    Library,
    Settings,
    Shortcuts,
    Panels,
    Files,
    Types,
    Ports,
    Connectors,
    Values,
    Refactor,
    DeleteModel,
    ProfileTags,
    Matrix,
    Viewpoints,
    Activities,
    StateMachines,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Menu,
    Toolbar,
    Keyboard,
}
impl Surface {
    pub fn name(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Toolbar => "toolbar",
            Self::Keyboard => "keyboard",
        }
    }
}
pub struct Spec {
    pub id: Id,
    pub key: &'static str,
    pub label: &'static str,
    pub menu: &'static str,
    pub shortcut: KeyboardShortcut,
}
const CMD: Modifiers = Modifiers::COMMAND;
const SHIFT: Modifiers = Modifiers { shift: true, ..CMD };
const ALT: Modifiers = Modifiers { alt: true, ..CMD };
const ALT_SHIFT: Modifiers = Modifiers { shift: true, ..ALT };
macro_rules! spec {
    ($id:ident,$key:literal,$label:literal,$menu:literal,$mods:ident,$keycode:ident) => {
        Spec {
            id: Id::$id,
            key: $key,
            label: $label,
            menu: $menu,
            shortcut: KeyboardShortcut::new($mods, Key::$keycode),
        }
    };
}
pub const MENUS: &[&str] = &["File", "Edit", "View", "Tools"];
pub const ALL: &[Spec] = &[
    spec!(StateMachines,"tools.state_machines","State machines and diagrams","Tools",ALT,U),
    spec!(Activities,"tools.activities","Activities and diagrams","Tools",ALT,H),
    spec!(Viewpoints,"view.viewpoints","Viewpoints and native card filters","View",ALT,L),
    spec!(Matrix,"tools.relation_matrix","Native relation matrix","Tools",ALT,M),
    spec!(ProfileTags,"tools.profile_tags","Exact profiles and literal tags","Tools",ALT,J),
    spec!(DeleteModel,"tools.semantic_delete","Delete model records with impact preview","Tools",ALT,G),
    spec!(Refactor,"tools.clone_move","Clone and move with reference preview","Tools",ALT,B),
    spec!(Values,"tools.values","Defaults and instance values","Tools",ALT,Y),
    spec!(Connectors,"tools.connectors","Connectors and conveyed items","Tools",ALT,K),
    spec!(Ports,"tools.ports","Ports and interfaces","Tools",ALT,I),
    spec!(Types,"tools.types","Types and properties","Tools",ALT,T),
    spec!(Files,"file.choose","Choose model file…","File",ALT,D),
    spec!(New, "file.new", "New", "File", CMD, N),
    spec!(Open, "file.open", "Open", "File", CMD, O),
    spec!(Recover, "file.recover", "Recover", "File", SHIFT, O),
    spec!(Import, "file.import_sysml", "Import SysML", "File", CMD, I),
    spec!(Save, "file.save", "Save", "File", CMD, S),
    spec!(
        Recovery,
        "file.recovery_copy",
        "Recovery copy",
        "File",
        SHIFT,
        S
    ),
    spec!(
        Export,
        "file.export_sysml",
        "Export SysML copy",
        "File",
        CMD,
        E
    ),
    spec!(
        Recoveries,
        "file.review_recoveries",
        "Review recovery copies",
        "File",
        SHIFT,
        R
    ),
    spec!(Quit, "file.quit", "Quit", "File", CMD, Q),
    spec!(Undo, "edit.undo", "Undo", "Edit", CMD, Z),
    spec!(Redo, "edit.redo", "Redo", "Edit", SHIFT, Z),
    spec!(
        LayoutUndo,
        "view.layout_undo",
        "Layout Undo",
        "View",
        ALT,
        Z
    ),
    spec!(
        LayoutRedo,
        "view.layout_redo",
        "Layout Redo",
        "View",
        ALT_SHIFT,
        Z
    ),
    spec!(
        Search,
        "edit.search",
        "Search native records",
        "Edit",
        CMD,
        F
    ),
    spec!(Usages, "edit.usages", "Find usages", "Edit", SHIFT, U),
    spec!(
        Impact,
        "edit.impact",
        "Inspect record impact",
        "Edit",
        SHIFT,
        I
    ),
    spec!(
        Presentation,
        "view.presentation_source",
        "Inspect presentation source…",
        "View",
        SHIFT,
        P
    ),
    spec!(
        External,
        "tools.external_projection",
        "Open external projection",
        "Tools",
        SHIFT,
        E
    ),
    spec!(
        Assurance,
        "tools.assurance",
        "Requirements and evidence",
        "Tools",
        SHIFT,
        A
    ),
    spec!(
        Library,
        "tools.library",
        "Browse library folder",
        "Tools",
        SHIFT,
        L
    ),
    spec!(
        Settings,
        "tools.recovery_settings",
        "Recovery settings",
        "Tools",
        CMD,
        Comma
    ),
    spec!(
        Shortcuts,
        "tools.shortcuts",
        "Keyboard shortcuts",
        "Tools",
        ALT,
        Comma
    ),
    spec!(Panels, "view.panels", "Panel layout", "View", ALT, P),
];
impl Id {
    pub fn spec(self) -> &'static Spec {
        ALL.iter()
            .find(|s| s.id == self)
            .expect("registered command")
    }
    pub fn is_history(self) -> bool {
        matches!(self, Self::Undo | Self::Redo)
    }
    pub fn is_layout_history(self) -> bool {
        matches!(self, Self::LayoutUndo | Self::LayoutRedo)
    }
}

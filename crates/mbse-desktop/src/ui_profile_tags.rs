//! Exact identity registration and literal metadata applications over existing model kinds.
use super::*;
use mbse_model::types::{
    self,
    tags::{self, Action},
};
#[derive(Default)]
pub(super) struct TagPanel {
    pub open: bool,
    note: String,
}
impl Workspace {
    fn tag_action(&mut self, action: Action) -> bool {
        let stamp = self.views.views.stamp(&self.session.editor);
        match self.views.views.dispatch(
            &mut self.session.editor,
            &stamp,
            mbse_model::native_views::NativeAction::Command(Command::Typed(
                types::Action::ProfileTag(action),
            )),
        ) {
            Ok(_) => {
                self.tag_panel.note="Profile metadata and unfinished form retained independently; native record kinds stay exact.".into();
                true
            }
            Err(e) => {
                self.tag_panel.note = e;
                false
            }
        }
    }
    pub(super) fn profile_tags_dialog(&mut self, ctx: &egui::Context) {
        if !self.tag_panel.open {
            return;
        }
        let instance = self.session.editor.freshness().instance_id;
        let mut open = true;
        let (mut close, mut raw, mut action, mut history) = (false, None, None, None);
        egui::Window::new("Exact profiles and literal tags").id(egui::Id::new("mbse-profile-tags")).open(&mut open).default_width(1000.).default_height(1050.).max_height((ctx.content_rect().height()-80.).max(240.)).vscroll(true).show(ctx,|ui|{
 ui.label("Profiles identify literal metadata by exact namespace, version, stereotype and tag names. Applications check an existing native/typed record kind; they never reclassify it or execute domain behavior. No namespace or version is inferred.");
 let store=match types::load(&self.session.editor.snapshot){Ok(s)=>s,Err(e)=>{ui.colored_label(ERROR,e);let r=ui.button("Close profiles");self.bound("tags:close",&r);close=r.clicked();return;}};
 ui.horizontal(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);for(redo,key,label,enabled)in[(false,"undo-model","Undo model",self.session.editor.can_undo()),(true,"redo-model","Redo model",self.session.editor.can_redo())]{let r=ui.add_enabled(ready&&enabled,egui::Button::new(label));self.bound(format!("tags:{key}"),&r);if r.clicked(){history=Some(redo);}}});
 let mut form=tags::form(&self.session.editor.snapshot).ok().flatten().unwrap_or_default();let before=form.clone();
 ui.label("Profile data: identity {namespace,version}; stereotypes map to applies_to record kinds and tags map to Boolean/Integer/Real/Text. Exact identity contents are immutable once registered; a changed definition needs a distinct version.");let r=ui.add(egui::TextEdit::multiline(&mut form.profile_json).desired_rows(3).desired_width(f32::INFINITY).id(egui::Id::new(("profile-tags",&instance,"profile-json"))));self.bound("tags:field:profile-json",&r);
 for(label,key,text)in[("Native or typed target ID","target",&mut form.target),("Profile namespace","namespace",&mut form.namespace),("Exact version","version",&mut form.version),("Stereotype name","stereotype",&mut form.stereotype)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("profile-tags",&instance,key))));self.bound(format!("tags:field:{key}"),&r);});}
 ui.label("Literal tag values: each exact tag name maps to {kind,text}; all declared keys required. Text and numeric spelling remain explicit.");let r=ui.add(egui::TextEdit::multiline(&mut form.values_json).desired_rows(2).desired_width(f32::INFINITY).id(egui::Id::new(("profile-tags",&instance,"values-json"))));self.bound("tags:field:values-json",&r);let edited=form!=before;if edited{raw=Some(Action::SetForm(form));}
 for(key,label,a)in[("register","Register exact profile",Action::Register),("apply","Apply exact stereotype and tags",Action::Apply),("remove","Remove exact application (keep raw form)",Action::Remove)]{let checked=tags::preview(&self.session.editor.snapshot,a.clone());let r=ui.add_enabled(!edited&&checked.is_ok(),egui::Button::new(label));self.bound(format!("tags:{key}"),&r);if r.clicked(){action=Some(a);}if let Err(e)=checked{ui.label(format!("{label}: {e}"));}}
 ui.separator();ui.label(format!("{} registered profiles · {} accepted applications",store.profiles.len(),store.profile_applications.len()));egui::ScrollArea::vertical().id_salt("profile-tag-inventory").max_height(180.).show(ui,|ui|{for p in store.profiles.values(){ui.label(format!("Exact identity {} · version {} · stereotypes {}",p.identity.namespace,p.identity.version,p.stereotypes.keys().cloned().collect::<Vec<_>>().join(", ")));}for(key,a)in store.profile_applications.iter().take(128){ui.label(format!("{} · underlying {:?} · {}",key,a.expected,serde_json::to_string(&a.values).unwrap_or_default()));}if store.profile_applications.len()>128{ui.label("Showing first128 applications; the complete accepted model remains authoritative.");}});ui.label(&self.tag_panel.note);let r=ui.button("Close profiles");self.bound("tags:close",&r);close=r.clicked();});
        self.tag_panel.open = open && !close;
        if let Some(a) = raw {
            self.tag_action(a);
        }
        if let Some(a) = action {
            self.tag_action(a);
        }
        if let Some(redo) = history {
            self.tag_panel.note = match self.typed_model_history(redo) {
                Ok(true) => {
                    "Model history applied; newer unfinished profile/tag form retained.".into()
                }
                Ok(false) => "No model history in this direction.".into(),
                Err(e) => e,
            };
        }
    }
    pub(super) fn profile_tags_audit(&self) -> serde_json::Value {
        let store = types::load(&self.session.editor.snapshot).ok();
        serde_json::json!({"schema":"mbse-craft.profile-tags-ui/1","open":self.tag_panel.open,"form":tags::form(&self.session.editor.snapshot).ok().flatten(),"profiles":store.as_ref().map(|s|&s.profiles),"applications":store.as_ref().map(|s|&s.profile_applications),"register_enabled":tags::preview(&self.session.editor.snapshot,Action::Register).is_ok(),"apply_enabled":tags::preview(&self.session.editor.snapshot,Action::Apply).is_ok(),"remove_enabled":tags::preview(&self.session.editor.snapshot,Action::Remove).is_ok(),"note":self.tag_panel.note})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rendered_register_apply_unknown_version_guard_and_undo_keep_native_kind_and_newer_raw() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p22/fixtures/native-typed.kernspan"
        ))
        .unwrap();
        let instance = session.editor.freshness().instance_id;
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(raw["snapshot"].clone()).unwrap(),
            instance,
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        let native = app.session.editor.snapshot.records.clone();
        let mut form=tags::Form{profile_json:r#"{"identity":{"namespace":"urn:mbse-craft:qa","version":"1.0"},"stereotypes":{"Valve":{"applies_to":["Measure"],"tags":{"label":"Text"}}}}"#.into(),target:"1003".into(),namespace:"urn:mbse-craft:qa".into(),version:"1.0".into(),stereotype:"Valve".into(),values_json:r#"{"label":{"kind":"Text","text":"prototype"}}"#.into()};
        assert!(app.tag_action(Action::SetForm(form.clone())));
        app.tag_panel.open = true;
        let render = |app: &mut Workspace, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(2200., 1600.),
                    )),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |_| app.profile_tags_dialog(&ctx),
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut app, vec![]);
        }
        let click = |app: &mut Workspace, key: &str| {
            let r = app.audit[key];
            let pos = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
            for pressed in [true, false] {
                render(
                    app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            render(app, vec![]);
        };
        click(&mut app, "tags:register");
        assert_eq!(
            types::load(&app.session.editor.snapshot)
                .unwrap()
                .profiles
                .len(),
            1
        );
        click(&mut app, "tags:apply");
        let accepted = types::load(&app.session.editor.snapshot)
            .unwrap()
            .profile_applications;
        assert_eq!(accepted.len(), 1);
        assert_eq!(app.session.editor.snapshot.records, native);
        form.version = "unknown".into();
        assert!(app.tag_action(Action::SetForm(form.clone())));
        render(&mut app, vec![]);
        assert!(!app.profile_tags_audit()["apply_enabled"].as_bool().unwrap());
        let before = app.session.editor.snapshot.clone();
        click(&mut app, "tags:apply");
        assert_eq!(app.session.editor.snapshot, before);
        assert_eq!(app.history_scope(), ShortcutTarget::Widget);
        assert!(
            app.command_reason(crate::command_catalog::Id::Save, &app.history_scope())
                .is_some()
        );
        click(&mut app, "tags:undo-model");
        assert!(
            types::load(&app.session.editor.snapshot)
                .unwrap()
                .profile_applications
                .is_empty()
        );
        assert_eq!(
            tags::form(&app.session.editor.snapshot).unwrap(),
            Some(form.clone())
        );
        click(&mut app, "tags:redo-model");
        assert_eq!(
            types::load(&app.session.editor.snapshot)
                .unwrap()
                .profile_applications,
            accepted
        );
        assert_eq!(
            tags::form(&app.session.editor.snapshot).unwrap(),
            Some(form)
        );
        let before = app.session.editor.snapshot.clone();
        click(&mut app, "tags:close");
        assert!(!app.tag_panel.open);
        assert_eq!(app.session.editor.snapshot, before);
        assert_eq!(app.session.editor.snapshot.records, native);
    }
}

//! Definition defaults, explicit Part overrides and read-only derived previews.
use super::*;
use mbse_model::types::{
    self,
    values::{self, Action, Target},
};
#[derive(Default)]
pub(super) struct ValuePanel {
    pub open: bool,
    selected: Option<Target>,
    derived: Option<values::Derived>,
    note: String,
}
impl Workspace {
    fn value_action(&mut self, a: Action) -> bool {
        let stamp = self.views.views.stamp(&self.session.editor);
        match self.views.views.dispatch(
            &mut self.session.editor,
            &stamp,
            mbse_model::native_views::NativeAction::Command(Command::Typed(types::Action::Value(
                a,
            ))),
        ) {
            Ok(_) => {
                self.value_panel.note =
                    "Value data and unfinished text retained independently.".into();
                true
            }
            Err(e) => {
                self.value_panel.note = e;
                false
            }
        }
    }
    pub(super) fn values_dialog(&mut self, ctx: &egui::Context) {
        if !self.value_panel.open {
            return;
        }
        let instance = self.session.editor.freshness().instance_id;
        let mut open = true;
        let mut close = false;
        let mut selected = None;
        let mut raw = None;
        let mut action = None;
        let mut derive = None;
        let mut history = None;
        egui::Window::new("Defaults, instance values and derived results").id(egui::Id::new("mbse-values")).open(&mut open).default_width(950.).default_height(950.).vscroll(true).max_height((ctx.content_rect().height()-80.).max(240.)).show(ctx,|ui|{
   ui.label("Definition defaults feed inheriting Parts. Explicit overrides stay independent. Read-only numeric previews never replace authored values; calculations here are unitless.");
   let store=match types::load(&self.session.editor.snapshot){Ok(s)=>s,Err(e)=>{ui.colored_label(ERROR,e);let r=ui.button("Close values");self.bound("values:close",&r);close=r.clicked();return;}};
   ui.horizontal(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);for(redo,key,label,enabled)in [(false,"undo-model","Undo model",self.session.editor.can_undo()),(true,"redo-model","Redo model",self.session.editor.can_redo())]{let r=ui.add_enabled(ready&&enabled,egui::Button::new(label));self.bound(format!("values:{key}"),&r);if r.clicked(){history=Some(redo);}}});
   ui.label("Definition property defaults");egui::ScrollArea::vertical().id_salt("value-default-rows").max_height(130.).show(ui,|ui|{for p in store.properties.values().filter(|p|p.multiplicity.lower==1&&p.multiplicity.upper==Some(1)){let target=Target::Default{property:p.id};let text=store.defaults.get(&p.id).map_or("unset".into(),|v|v.text.clone());let r=ui.selectable_label(self.value_panel.selected==Some(target),format!("Property#{} · {} · owner#{} · default {}",p.id.0,p.name,p.owner.0,text));self.bound(format!("values:default:{}",p.id.0),&r);if r.clicked(){selected=Some(target);}}});
   ui.label("Part instance values · origin stays explicit");egui::ScrollArea::vertical().id_salt("value-instance-rows").max_height(180.).show(ui,|ui|{let mut count=0;for usage in store.usages.values().filter(|u|u.kind==types::UsageKind::Part){for p in store.properties.values().filter(|p|p.owner==usage.target&&p.multiplicity.lower==1&&p.multiplicity.upper==Some(1)){if count>=128{break}count+=1;let target=Target::Override{usage:usage.id,property:p.id};let effective=values::effective(&store,target).unwrap();let r=ui.selectable_label(self.value_panel.selected==Some(target),format!("Part#{} · Property#{} · {:?} · {}",usage.id.0,p.id.0,effective.origin,effective.value.as_ref().map_or("unset",|v|v.text.as_str())));self.bound(format!("values:override:{}:{}",usage.id.0,p.id.0),&r);if r.clicked(){selected=Some(target);}}}if count==128{ui.label("Showing the first 128 instance value choices.");}});
   let forms=values::forms(&self.session.editor.snapshot).unwrap_or_default();if let Some((target,mut form))=self.value_panel.selected.and_then(|target|forms.get(&target.key()).cloned().map(|f|(target,f))){ui.separator();ui.label(match target{Target::Default{property}=>format!("Definition default · Property#{} · unfinished text retained",property.0),Target::Override{usage,property}=>format!("Part#{} · Property#{} · unfinished override text retained",usage.0,property.0)});let before=form.clone();for(label,key,text)in [("Authored value","value",&mut form.text),("Read-only expression in x","expression",&mut form.expression)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-value",&instance,target.key(),key))));self.bound(format!("values:field:{key}"),&r);});}let edited=form!=before;if edited{raw=Some(Action::Set(form.clone()));}
    let checked=values::preview(&self.session.editor.snapshot,target);if let Err(e)=&checked{ui.colored_label(ERROR,format!("Unapplied value retained: {e}"));}let r=ui.add_enabled(checked.is_ok()&&!edited,egui::Button::new("Apply authored value"));self.bound("values:apply",&r);if r.clicked(){action=Some(Action::Apply(target));}
    let explicit=match target{Target::Default{property}=>store.defaults.contains_key(&property),Target::Override{usage,property}=>store.overrides.get(&usage).is_some_and(|m|m.contains_key(&property))};let r=ui.add_enabled(explicit,egui::Button::new(if matches!(target,Target::Override{..}){"Use default (clear explicit override)"}else{"Clear authored default"}));self.bound("values:clear",&r);if r.clicked(){action=Some(Action::Clear(target));}
    match values::effective(&store,target){Ok(e)=>{ui.label(format!("Accepted effective value · {:?} · {}",e.origin,e.value.as_ref().map_or("unset",|v|v.text.as_str())));},Err(e)=>{ui.colored_label(ERROR,e);}}
    let r=ui.add_enabled(!edited,egui::Button::new("Calculate read-only preview"));self.bound("values:derive",&r);if r.clicked(){derive=Some((target,form.expression));}
   }
   if let Some(report)=&self.value_panel.derived{let current=report.current(&self.session.editor);if let Some(error)=report.diagnostic(){ui.colored_label(ERROR,error.to_string());}let text=format!("Read-only derived result: {} · {:?} source · {}",report.value().map_or("unavailable".into(),|n|n.to_string()),report.effective().origin,if current{"current"}else{"outdated; calculate again"});ui.colored_label(if current{ACCENT}else{ERROR},text);ui.label("Uses accepted effective values. Unapplied value text is excluded; results are never applied to defaults or overrides.");}
   ui.label(&self.value_panel.note);let r=ui.button("Close values");self.bound("values:close",&r);close=r.clicked();
  });
        self.value_panel.open = open && !close;
        if let Some(target) = selected {
            if self.value_action(Action::Begin(target)) {
                self.value_panel.selected = Some(target);
                self.value_panel.derived = None;
            }
        }
        if let Some(a) = raw {
            self.value_action(a);
        }
        if let Some(a) = action {
            self.value_action(a);
        }
        if let Some((target, expression)) = derive {
            match values::derive(&self.session.editor, target, &expression) {
                Ok(report) => {
                    self.value_panel.derived = Some(report);
                    self.value_panel.note =
                        "Read-only calculation retained separately from authored data.".into();
                }
                Err(e) => {
                    self.value_panel.note = e;
                    self.value_panel.derived = None;
                }
            }
        }
        if let Some(redo) = history {
            self.value_panel.note = match self.typed_model_history(redo) {
                Ok(true) => "Model history applied; current value/expression text retained.".into(),
                Ok(false) => "No model history in this direction.".into(),
                Err(e) => e,
            };
        }
    }
    pub(super) fn values_audit(&self) -> serde_json::Value {
        let store = types::load(&self.session.editor.snapshot).ok();
        let effective = store.as_ref().map(|s| {
            let mut rows = serde_json::Map::new();
            for p in s.properties.values() {
                if rows.len() >= 512 {
                    break;
                }
                let t = Target::Default { property: p.id };
                if let Ok(e) = values::effective(s, t) {
                    rows.insert(t.key(), serde_json::to_value(e).unwrap());
                }
            }
            for u in s.usages.values() {
                if rows.len() >= 512 {
                    break;
                }
                for p in s.properties.values().filter(|p| p.owner == u.target) {
                    if rows.len() >= 512 {
                        break;
                    }
                    let t = Target::Override {
                        usage: u.id,
                        property: p.id,
                    };
                    if let Ok(e) = values::effective(s, t) {
                        rows.insert(t.key(), serde_json::to_value(e).unwrap());
                    }
                }
            }
            rows
        });
        serde_json::json!({"schema":"mbse-craft.values-ui/1","open":self.value_panel.open,"selected":self.value_panel.selected,"store":store,"forms":values::forms(&self.session.editor.snapshot).ok(),"effective":effective,"derived":self.value_panel.derived,"derived_current":self.value_panel.derived.as_ref().map(|r|r.current(&self.session.editor)),"note":self.value_panel.note})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_derived_button_never_writes_values_and_newer_invalid_text_keeps_focus_and_marks_result_stale()
     {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p19/fixtures/connectors.kernspan"
        ))
        .unwrap();
        let instance = session.editor.freshness().instance_id;
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(raw["snapshot"].clone()).unwrap(),
            instance,
        )
        .unwrap();
        for action in [
            types::Action::BeginProperty {
                owner: types::Id(1007),
            },
            types::Action::SetForm {
                id: types::Id(1020),
                form: types::Form {
                    owner: types::Id(1007),
                    name: "gain".into(),
                    value_type: "1005".into(),
                    lower: "1".into(),
                    upper: "1".into(),
                },
            },
            types::Action::ApplyProperty {
                id: types::Id(1020),
            },
        ] {
            session
                .editor
                .command(&session.editor.freshness(), Command::Typed(action))
                .unwrap();
        }
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        let target = Target::Default {
            property: types::Id(1020),
        };
        assert!(app.value_action(Action::Begin(target)));
        assert!(app.value_action(Action::Set(values::Form {
            target,
            text: "12.5".into(),
            expression: "x*2".into()
        })));
        assert!(app.value_action(Action::Apply(target)));
        app.value_panel.open = true;
        app.value_panel.selected = Some(target);
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
                |_| app.values_dialog(&ctx),
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut app, vec![]);
        }
        let before = app.session.editor.snapshot.clone();
        let rect = app.audit["values:derive"];
        let pos = egui::pos2((rect[0] + rect[2]) / 2., (rect[1] + rect[3]) / 2.);
        for pressed in [true, false] {
            render(
                &mut app,
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
        assert_eq!(app.value_panel.derived.as_ref().unwrap().value(), Some(25.));
        assert!(
            app.value_panel
                .derived
                .as_ref()
                .unwrap()
                .current(&app.session.editor)
        );
        assert_eq!(app.session.editor.snapshot, before);
        let field = egui::Id::new((
            "mbse-value",
            &app.session.editor.freshness().instance_id,
            target.key(),
            "value",
        ));
        assert!(ctx.read_response(field).unwrap().enabled());
        ctx.memory_mut(|m| m.request_focus(field));
        let f = values::Form {
            target,
            text: "2e+".into(),
            expression: "x*2".into(),
        };
        assert!(app.value_action(Action::Set(f.clone())));
        render(&mut app, vec![]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert_eq!(
            values::forms(&app.session.editor.snapshot).unwrap()[&target.key()],
            f
        );
        assert!(
            !app.value_panel
                .derived
                .as_ref()
                .unwrap()
                .current(&app.session.editor)
        );
        assert_eq!(
            types::load(&app.session.editor.snapshot).unwrap().defaults[&types::Id(1020)].text,
            "12.5"
        );
        assert_eq!(app.history_scope(), ShortcutTarget::Widget);
        assert!(
            app.command_reason(crate::command_catalog::Id::Save, &app.history_scope())
                .is_some()
        );
    }
}

//! Connector routes bind exact Part-qualified port occurrence identities.
use super::*;
use mbse_model::{
    native_views::{Geometry, Layout, connectors as vc, ports as vp},
    types::connectors::Endpoint,
};
impl Workspace {
    pub(super) fn qualified_port_controls(
        &mut self,
        ui: &mut egui::Ui,
        store: &mbse_model::types::Store,
    ) {
        let ready = !self.dock_blocked();
        let mut action = None;
        let mut reuse_choices = 0;
        let mut route_choices = 0;
        let header=egui::CollapsingHeader::new("Part-qualified ports and connector routes").id_salt("qualified-port-controls").default_open(true).show(ui,|ui|{
   ui.label("Route an accepted connector between exact Part/Port occurrences. Removing a route keeps the semantic connector.");
   ui.horizontal_wrapped(|ui|{for usage in store.usages.values().filter(|u|u.kind==mbse_model::types::UsageKind::Part){for port in store.ports.values().filter(|p|p.owner==usage.target){if reuse_choices>=96{break}reuse_choices+=1;let r=ui.add_enabled(ready,egui::Button::new(format!("Reuse Part#{} Port#{}",usage.id.0,port.id.0)));self.bound(format!("connectors:reuse:{}:{}",usage.id.0,port.id.0),&r);if r.clicked(){let doc=self.views.views.document();let index=doc.typed_ports.values().filter(|o|o.view==self.views.active&&o.usage.is_some()).count();action=Some(Layout::Port(vp::Layout::ReuseInUsage{view:self.views.active,port:port.id,usage:usage.id,geometry:Geometry::at(25.+(index%2)as f64*350.,315.+(index/2)as f64*160.)}));}}}});
   let doc=self.views.views.document();for connector in store.connectors.values(){let sources:Vec<_>=doc.typed_ports.values().filter(|o|o.view==self.views.active&&o.presence.active()&&o.visible&&o.usage.map(|usage|Endpoint{usage,port:o.port})==Some(connector.source)).collect();let targets:Vec<_>=doc.typed_ports.values().filter(|o|o.view==self.views.active&&o.presence.active()&&o.visible&&o.usage.map(|usage|Endpoint{usage,port:o.port})==Some(connector.target)).collect();ui.label(format!("Connector#{} · {} · conveyed#{}",connector.id.0,connector.name,connector.conveyed.0));
    if sources.is_empty()||targets.is_empty(){ui.label("Reuse both qualified endpoint ports in this view to route this connector.");}
    for source in &sources{if route_choices>=128{break}for target in &targets{if route_choices>=128{break}route_choices+=1;let exists=doc.connector_edges.values().any(|e|e.view==self.views.active&&e.connector==connector.id&&e.source==source.id&&e.target==target.id);let r=ui.add_enabled(ready&&!exists,egui::Button::new(format!("Route connector#{} · occurrence {} to {}",connector.id.0,source.id.get(),target.id.get())));self.bound(format!("connectors:route:{}:{}:{}",connector.id.0,source.id.get(),target.id.get()),&r);if r.clicked(){action=Some(Layout::Connector(vc::Layout::Route{view:self.views.active,connector:connector.id,source:source.id,target:target.id}));}}}
   }
   if reuse_choices==96||route_choices==128{ui.label("Showing the first 96 Part/Port pairs and 128 route choices.");}
   for edge in doc.connector_edges.values().filter(|e|e.view==self.views.active){ui.horizontal_wrapped(|ui|{ui.label(format!("Route {} · connector#{} · {}",edge.id.get(),edge.connector.0,if edge.presence.active(){"active"}else{"detached"}));let r=ui.add_enabled(ready,egui::Button::new(if edge.visible{"Hide route"}else{"Show route"}));self.bound(format!("connectors:edge:{}:visibility",edge.id.get()),&r);if r.clicked(){action=Some(Layout::Connector(vc::Layout::Visible(edge.id,!edge.visible)));}let r=ui.add_enabled(ready,egui::Button::new("Remove route"));self.bound(format!("connectors:edge:{}:remove",edge.id.get()),&r);if r.clicked(){action=Some(Layout::Connector(vc::Layout::Remove(edge.id)));}});}
  });
        self.bound("connectors:controls", &header.header_response);
        if let Some(action) = action {
            let stamp = self.views.views.stamp(&self.session.editor);
            self.views.note=match self.views.views.apply(&self.session.editor,&stamp,action){Ok(())=>"Qualified presentation updated; connector semantics unchanged. Layout Undo is separate.".into(),Err(error)=>error};
        }
    }
    pub(super) fn paint_connector_routes(
        &mut self,
        ui: &mut egui::Ui,
        area: Rect,
        camera: &mbse_model::native_views::Camera,
        scale: f32,
    ) {
        let doc = self.views.views.document();
        let Ok(store) = mbse_model::types::load(&self.session.editor.snapshot) else {
            return;
        };
        for edge in doc
            .connector_edges
            .values()
            .filter(|e| e.view == self.views.active && e.visible && e.presence.active())
        {
            let Some(a) = doc
                .typed_ports
                .get(&edge.source)
                .filter(|o| o.visible && o.presence.active())
            else {
                continue;
            };
            let Some(b) = doc
                .typed_ports
                .get(&edge.target)
                .filter(|o| o.visible && o.presence.active())
            else {
                continue;
            };
            let Some(c) = store.connectors.get(&edge.connector) else {
                continue;
            };
            let point = |g: &Geometry| {
                area.min
                    + Vec2::new(
                        (g.x + g.width / 2. - camera.x) as f32,
                        (g.y + g.height / 2. - camera.y) as f32,
                    ) * scale
            };
            let source = point(&a.geometry);
            let target = point(&b.geometry);
            let direction = target - source;
            if direction.length() < 1. {
                continue;
            }
            let unit = direction.normalized();
            let extent = |g: &Geometry| {
                ((g.width as f32 * 0.5) / unit.x.abs().max(0.000001))
                    .min((g.height as f32 * 0.5) / unit.y.abs().max(0.000001))
                    * scale
            };
            let start = source + unit * (extent(&a.geometry) + 3.);
            let end = target - unit * (extent(&b.geometry) + 3.);
            ui.painter()
                .arrow(start, end - start, Stroke::new(2., ACCENT));
            let center = (start + end.to_vec2()) * 0.5;
            let label = format!("{} · Value#{}", c.name, c.conveyed.0);
            let galley =
                ui.painter()
                    .layout_no_wrap(label, FontId::proportional(12. * scale), ACCENT);
            let rect = Rect::from_center_size(center, galley.size() + Vec2::new(10., 6.));
            ui.painter().rect_filled(rect, 3., PAPER);
            ui.painter()
                .galley(rect.min + Vec2::new(5., 3.), galley, ACCENT);
        }
    }
    pub(super) fn connector_routes_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.connector-routes-ui/1","profile":self.views.views.document().profile,"edges":self.views.views.document().connector_edges})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_route_controls_disambiguate_two_parts_of_one_type_and_remove_only_presentation() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p18b/fixtures/connectors.kernspan"
        ))
        .unwrap();
        let instance = session.editor.freshness().instance_id;
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
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
        let before = app.session.editor.snapshot.clone();
        let view = app.views.active;
        let mut occurrences = Vec::new();
        for (usage, port, x) in [(1009, 1017, 25.), (1010, 1015, 400.), (1009, 1015, 800.)] {
            let id = app.views.views.document().next_id();
            let stamp = app.views.views.stamp(&app.session.editor);
            app.views
                .views
                .apply(
                    &app.session.editor,
                    &stamp,
                    Layout::Port(vp::Layout::ReuseInUsage {
                        view,
                        usage: mbse_model::types::Id(usage),
                        port: mbse_model::types::Id(port),
                        geometry: Geometry::at(x, 200.),
                    }),
                )
                .unwrap();
            occurrences.push(id);
        }
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
                |ui| {
                    let store = mbse_model::types::load(&app.session.editor.snapshot).unwrap();
                    app.qualified_port_controls(ui, &store);
                },
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut app, vec![]);
        }
        let key = format!(
            "connectors:route:1018:{}:{}",
            occurrences[0], occurrences[1]
        );
        assert!(app.audit.contains_key(&key));
        assert!(!app.audit.contains_key(&format!(
            "connectors:route:1018:{}:{}",
            occurrences[0], occurrences[2]
        )));
        let click = |app: &mut Workspace, key: &str| {
            let rect = app.audit[key];
            let pos = egui::pos2((rect[0] + rect[2]) / 2., (rect[1] + rect[3]) / 2.);
            render(
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            render(
                app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            render(app, vec![]);
        };
        click(&mut app, &key);
        assert_eq!(app.views.views.document().connector_edges.len(), 1);
        let edge = app
            .views
            .views
            .document()
            .connector_edges
            .values()
            .next()
            .unwrap()
            .clone();
        assert_eq!(edge.source.get(), occurrences[0]);
        assert_eq!(edge.target.get(), occurrences[1]);
        click(
            &mut app,
            &format!("connectors:edge:{}:remove", edge.id.get()),
        );
        assert!(app.views.views.document().connector_edges.is_empty());
        assert_eq!(app.session.editor.snapshot, before);
    }
}

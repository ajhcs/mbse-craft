//! Port cards reuse semantic IDs with independent occurrence geometry and history.
use super::*;
use mbse_model::native_views::{Geometry, Layout, ports as vp};
impl Workspace {
    fn port_layout(&mut self, action: vp::Layout) {
        if self.dock_blocked() {
            self.views.note="Finish pending writes, gestures or auxiliary editors before editing port presentation.".into();
            return;
        }
        let stamp = self.views.views.stamp(&self.session.editor);
        self.views.note = match self.views.views.apply(
            &self.session.editor,
            &stamp,
            Layout::Port(action),
        ) {
            Ok(()) => {
                "Port presentation updated; semantic contract unchanged. Layout Undo is separate."
                    .into()
            }
            Err(error) => error,
        };
    }
    pub(super) fn port_view_controls(&mut self, ui: &mut egui::Ui) {
        if self.session.editor.snapshot.extensions.is_none() {
            return;
        }
        let store = match mbse_model::types::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(_) => return,
        };
        let rows: Vec<_> = self
            .views
            .views
            .document()
            .typed_ports
            .values()
            .filter(|o| o.view == self.views.active)
            .cloned()
            .collect();
        if store.ports.is_empty() && rows.is_empty() {
            return;
        }
        let ready = !self.dock_blocked();
        let mut action = None;
        egui::CollapsingHeader::new("Typed ports and occurrences").id_salt("typed-port-controls").default_open(true).show(ui,|ui|{
   ui.label("Reuse one port contract across views. Move, hide, pin and remove affect this presentation only.");
   ui.horizontal_wrapped(|ui|{for p in store.ports.values(){let r=ui.add_enabled(ready,egui::Button::new(format!("Reuse port #{} · {}",p.id.0,p.name)));self.bound(format!("ports:reuse:{}",p.id.0),&r);if r.clicked(){let doc=self.views.views.document();let y=doc.occurrences.values().filter(|o|o.view==self.views.active).map(|o|o.geometry.y+o.geometry.height+24.).chain(rows.iter().map(|o|o.geometry.y+o.geometry.height+24.)).fold(24.,f64::max);action=Some(vp::Layout::Reuse{view:self.views.active,port:p.id,geometry:Geometry::at(25.,y)});}}});
   for o in &rows{ui.horizontal_wrapped(|ui|{ui.label(format!("Port #{} · occurrence {} · {}{}",o.port.0,o.id.get(),if o.presence.active(){"active"}else{"detached"},if o.pinned{" · pinned"}else{""}));let r=ui.add_enabled(ready,egui::Button::new(if o.visible{"Hide"}else{"Show"}));self.bound(format!("ports:occurrence:{}:visibility",o.id.get()),&r);if r.clicked(){action=Some(vp::Layout::Visible(o.id,!o.visible));}let r=ui.add_enabled(ready,egui::Button::new(if o.pinned{"Unpin"}else{"Pin"}));self.bound(format!("ports:occurrence:{}:pin",o.id.get()),&r);if r.clicked(){action=Some(vp::Layout::Pin(o.id,!o.pinned));}let r=ui.add_enabled(ready&&!o.pinned,egui::Button::new("Move right 20"));self.bound(format!("ports:occurrence:{}:move-right",o.id.get()),&r);if r.clicked(){let mut geometry=o.geometry.clone();geometry.x+=20.;action=Some(vp::Layout::Move(o.id,geometry));}let r=ui.add_enabled(ready,egui::Button::new("Remove occurrence"));self.bound(format!("ports:occurrence:{}:remove",o.id.get()),&r);if r.clicked(){action=Some(vp::Layout::Remove(o.id));}});}
  });
        if let Some(action) = action {
            self.port_layout(action);
        }
    }
    pub(super) fn paint_port_occurrences(
        &mut self,
        ui: &mut egui::Ui,
        area: Rect,
        camera: &mbse_model::native_views::Camera,
        scale: f32,
        ports: &[vp::PortOccurrence],
    ) {
        if ports.is_empty() {
            return;
        }
        let store = match mbse_model::types::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(_) => return,
        };
        for o in ports {
            let Some(port) = store.ports.get(&o.port) else {
                continue;
            };
            let rect = Rect::from_min_size(
                area.min
                    + Vec2::new(
                        (o.geometry.x - camera.x) as f32,
                        (o.geometry.y - camera.y) as f32,
                    ) * scale,
                Vec2::new(o.geometry.width as f32, o.geometry.height as f32) * scale,
            );
            ui.painter().rect(
                rect,
                7.,
                PAPER,
                Stroke::new(1.5, ACCENT),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.min + Vec2::new(14., 13.) * scale,
                egui::Align2::LEFT_TOP,
                format!("Port #{} · occurrence {}", port.id.0, o.id.get()),
                FontId::proportional(12. * scale),
                ACCENT,
            );
            let label = format!(
                "{}\nInterface #{} · assigned {}",
                port.name,
                port.contract.0,
                port.assignment
                    .map_or("declaration".into(), |id| format!("#{}", id.0))
            );
            let galley = ui.painter().layout(
                label,
                FontId::proportional(15. * scale),
                INK,
                (rect.width() - 28. * scale).max(1.),
            );
            ui.painter().with_clip_rect(rect.shrink(5.)).galley(
                rect.min + Vec2::new(14., 36.) * scale,
                galley,
                INK,
            );
            let r = ui.interact(
                rect,
                ui.id().with(("port-occurrence", o.id.get())),
                egui::Sense::click(),
            );
            self.bound(format!("port-occurrence:{}", o.id.get()), &r);
            if r.clicked() && !self.dock_blocked() {
                self.open_port_editor(o.port);
            }
        }
    }
    pub(super) fn port_presentation_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.port-presentation-ui/1","profile":self.views.views.document().profile,"active_view":self.views.active,"occurrences":self.views.views.document().typed_ports})
    }
}

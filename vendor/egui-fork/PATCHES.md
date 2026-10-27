# Craft suite patches on egui 0.36.2 (`craft/0.36.2`)

Base: upstream tag `0.36.2` (`49682f8b`); eframe, egui-wgpu and egui-winit sources match crates.io
0.36.2 exactly. Apps patch every egui-workspace crate they use to this checkout, so only one egui
resolves. One commit per patch; drop it when an egui release in use contains it.

| # | Commit | Crate | Change | Upstream | Source in the suite |
|---|---|---|---|---|---|
| E1 | `e7e37ad` | eframe | `NativeOptions::viewport_hooks` (`ViewportHooks`): each viewport's native window on create/remove, and every viewport's raw input before egui (immediate viewports included) | not proposed upstream yet | needed by craft-tablet (native Wayland pen input) |
| E2+E3 | `b02321d` | egui-wgpu | fit a surface within `max_texture_dimension_2d`; first configure and renderer creation inside wgpu error scopes, returned as `WgpuError::ConfigureSurface` / `CreateRenderer` | not proposed | pdfcraft#609 (#577), #617 (#519), #1042; gridcraft#761 |
| E4 | `c01acc2` | egui-winit | use winit's Wayland clipboard when available instead of starting smithay-clipboard (Hyprland serves only one data device per client) | emilk/egui#8657 (draft), ported to 0.36.2 | companion of winit W15 |

E4 was ported by hand: on 0.36.2 `Clipboard::new` is a safe fn with a different body, so the early
return was placed before the struct literal; `get`/`set_text` changes applied as in the PR.

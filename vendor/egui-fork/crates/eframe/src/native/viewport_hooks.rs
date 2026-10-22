//! Per-viewport native window and raw-input hooks.
//!
//! eframe exposes the root window's handles through [`crate::CreationContext`] only, and runs
//! [`crate::App::raw_input_hook`] for the root and deferred viewports but not for immediate ones.
//! Integrations that need every viewport's native window (for example to match Wayland surfaces
//! named by a protocol the windowing layer doesn't implement) or that must add input to every
//! viewport before egui processes it can set [`crate::NativeOptions::viewport_hooks`].

use std::sync::{Arc, PoisonError, RwLock};

/// Callbacks eframe makes for every viewport. All methods have empty defaults.
///
/// They run on the event loop thread, inside eframe's frame handling: keep them short and don't
/// call back into eframe or block.
pub trait ViewportHooks: Send + Sync {
    /// eframe created the native window of `viewport` (root, deferred or immediate).
    fn window_created(&self, _viewport: egui::ViewportId, _window: &winit::window::Window) {}

    /// eframe dropped `viewport`; its window is about to be (or has been) destroyed.
    fn window_destroyed(&self, _viewport: egui::ViewportId) {}

    /// The raw input of one viewport (`raw_input.viewport_id`), before egui processes it. Runs for
    /// root, deferred and immediate viewports, before [`crate::App::raw_input_hook`].
    fn raw_input(&self, _ctx: &egui::Context, _raw_input: &mut egui::RawInput) {}
}

static HOOKS: RwLock<Option<Arc<dyn ViewportHooks>>> = RwLock::new(None);

pub(crate) fn install(hooks: Option<Arc<dyn ViewportHooks>>) {
    *HOOKS.write().unwrap_or_else(PoisonError::into_inner) = hooks;
}

pub(crate) fn with(f: impl FnOnce(&dyn ViewportHooks)) {
    let hooks = HOOKS.read().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(hooks) = hooks {
        f(&*hooks);
    }
}

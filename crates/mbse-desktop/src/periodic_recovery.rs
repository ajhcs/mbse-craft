//! A clock-driven request policy, never a preservation acknowledgment.
//! Busy requests coalesce: observation continues, but no ticket is replaced.
use crate::views_ui::ViewWorkspace;
use mbse_model::{native_views::Stamp, stage1_io::Session};
use std::time::{Duration, Instant};

pub struct PeriodicRecovery {
    observed: Stamp,
    last_edit: Instant,
    last_attempt: Instant,
    verified: Option<(Stamp, bool, bool)>,
}
impl PeriodicRecovery {
    pub fn new(now: Instant, session: &Session, views: &ViewWorkspace) -> Self {
        Self {
            observed: views.views.stamp(&session.editor),
            last_edit: now,
            last_attempt: now,
            verified: None,
        }
    }
    /// Cached point-in-time evidence for display, never navigation permission.
    pub fn status(&self, session: &Session, views: &ViewWorkspace) -> (&'static str, &'static str) {
        match &self.verified {
            Some((stamp, native, layout)) if *stamp == views.views.stamp(&session.editor) => (
                if *native {
                    "verified at timer check"
                } else {
                    "not verified at timer check"
                },
                if *layout {
                    "verified at timer check"
                } else {
                    "not verified at timer check"
                },
            ),
            _ => ("awaiting timer check", "awaiting timer check"),
        }
    }
    pub fn reset_interval(&mut self, now: Instant) {
        self.last_attempt = now;
    }
    pub fn due(
        &mut self,
        now: Instant,
        enabled: bool,
        interval: Duration,
        navigation_pending: bool,
        session: &Session,
        views: &ViewWorkspace,
    ) -> bool {
        let stamp = views.views.stamp(&session.editor);
        if self.observed != stamp {
            self.observed = stamp;
            self.last_edit = now;
            self.verified = None;
        }
        if views.layout_gesture_active() {
            self.last_edit = now;
            return false;
        }
        if !enabled
            || navigation_pending
            || session.pending()
            || views.pair_pending()
            || views.io.pending()
            || now.saturating_duration_since(self.last_edit) < Duration::from_millis(750)
            || now.saturating_duration_since(self.last_attempt) < interval
        {
            return false;
        }
        // Bounded artifact verification runs at most once per interval, even
        // when the acknowledged pair is a no-op or verification fails.
        self.last_attempt = now;
        let native_current = session.recovery_current();
        let layout_current = views.preserved(session, &mbse_model::stage1_io::WriteKind::Recovery)
            && views
                .verify_preserved(session, &mbse_model::stage1_io::WriteKind::Recovery)
                .is_ok();
        self.verified = Some((self.observed.clone(), native_current, layout_current));
        let saved_pair = !session.is_dirty()
            && views.preserved(session, &mbse_model::stage1_io::WriteKind::Save)
            && views
                .verify_preserved(session, &mbse_model::stage1_io::WriteKind::Save)
                .is_ok();
        !saved_pair && !(native_current && layout_current)
    }
}

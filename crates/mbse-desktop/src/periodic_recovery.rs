//! A clock-driven request policy, never a preservation acknowledgment.
//! Busy requests coalesce: observation continues, but no ticket is replaced.
use crate::views_ui::ViewWorkspace;
use mbse_model::{native_views::Stamp, stage1_io::Session};
use std::time::{Duration, Instant};

pub struct PeriodicRecovery {
    observed: Stamp,
    last_edit: Instant,
    last_attempt: Instant,
}
impl PeriodicRecovery {
    pub fn new(now: Instant, session: &Session, views: &ViewWorkspace) -> Self {
        Self {
            observed: views.views.stamp(&session.editor),
            last_edit: now,
            last_attempt: now,
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
        // These layout hints only schedule work; they never authorize leaving
        // a document. Close/open retain their exact-artifact verification.
        let native_needed = session.is_dirty() && !session.recovery_current();
        let layout_needed = !views.io.is_saved(session, &views.views)
            && !views.io.recovery_current(session, &views.views);
        if !native_needed && !layout_needed {
            return false;
        }
        // Throttle failed preparation/start as well as successful attempts.
        // Merely waiting for somebody else's pair does not consume an interval.
        self.last_attempt = now;
        true
    }
}

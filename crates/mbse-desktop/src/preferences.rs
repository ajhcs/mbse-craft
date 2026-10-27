//! App-owned opt-in policy; persistence is the audited upstream craft-settings store.
use craft_settings::{Location, Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecoveryPreferences {
    pub periodic_recovery: bool,
    pub recovery_interval_seconds: u64,
    pub shortcut_overrides: crate::shortcuts::Overrides,
    pub workspace_layout: String,
    pub workspace_perspectives: std::collections::BTreeMap<String,String>,
}
impl Default for RecoveryPreferences {
    fn default() -> Self {
        Self {
            periodic_recovery: false,
            recovery_interval_seconds: 60,
            shortcut_overrides: Default::default(),
            workspace_layout: String::new(),
            workspace_perspectives: Default::default(),
        }
    }
}
impl Settings for RecoveryPreferences {
    const APP_ID: &'static str = "mbse-craft";
    const SCHEMA_VERSION: u32 = 1;
}

enum PreferenceWrite {
    Saved(String),
    Failed(String),
}
pub struct Preferences {
    pub value: RecoveryPreferences,
    pub report: String,
    pub status: String,
    pub persistent: bool,
    pub read_only: bool,
    pub load_warning: bool,
    sender: Option<Sender<(u64, RecoveryPreferences)>>,
    receiver: Receiver<(u64, PreferenceWrite)>,
    worker: Option<JoinHandle<()>>,
    generation: u64,
    acknowledged: u64,
}
impl Preferences {
    pub fn open(config: Option<PathBuf>, ctx: eframe::egui::Context) -> Self {
        let (mut store, report) = SettingsStore::<RecoveryPreferences>::open(
            config.map(Location::Dir).unwrap_or(Location::Memory),
        );
        let persistent = store.is_persistent();
        let read_only = store.is_read_only();
        let load_warning = report.has_problems();
        let mut value = store.get().clone();
        let out_of_bounds = !(15..=3600).contains(&value.recovery_interval_seconds);
        value.recovery_interval_seconds = value.recovery_interval_seconds.clamp(15, 3600);
        let report = format!(
            "{}{}{}",
            if persistent {
                format!(
                    "Settings: {}",
                    report
                        .dir
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                )
            } else {
                "Settings are in memory for this session; use --config-dir for persistence.".into()
            },
            if report.has_problems() {
                format!("; load warning: {report:?}")
            } else {
                String::new()
            },
            if out_of_bounds {
                "; recovery interval clamped to 15..3600 seconds"
            } else {
                ""
            }
        );
        let (sender, updates) = mpsc::channel::<(u64, RecoveryPreferences)>();
        let (results, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut deadline: Option<Instant> = None;
            let mut generation = 0;
            loop {
                let message = match deadline {
                    Some(due) => updates.recv_timeout(
                        due.saturating_duration_since(Instant::now())
                            .min(Duration::from_secs(60)),
                    ),
                    None => updates
                        .recv()
                        .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
                };
                match message {
                    Ok((next_generation, mut value)) => {
                        generation = next_generation;
                        for next in updates.try_iter() {
                            generation = next.0;
                            value = next.1;
                        }
                        store.update(|prefs| *prefs = value);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Shutdown gets one final flush, never an unbounded retry loop.
                        if store.is_dirty() {
                            if let Err(error) = store.flush() {
                                eprintln!("MBSE-Craft preferences exit flush failed: {error}");
                            }
                        }
                        break;
                    }
                }
                let tick = store.tick(Instant::now());
                deadline = tick.next_deadline;
                let event = match tick.status {
                    craft_settings::SaveStatus::Saved => Some(PreferenceWrite::Saved({
                        let moved = store.take_moved_aside();
                        if moved.is_empty() {
                            "Preferences saved.".into()
                        } else {
                            format!("Preferences saved; damaged file retained at {moved:?}")
                        }
                    })),
                    craft_settings::SaveStatus::Failed(error) => Some(PreferenceWrite::Failed(
                        format!("{error}; the shared settings policy will retry after backoff."),
                    )),
                    craft_settings::SaveStatus::Idle if !store.is_dirty() => Some(
                        PreferenceWrite::Saved("Session preferences applied.".into()),
                    ),
                    _ => None,
                };
                if let Some(event) = event {
                    if results.send((generation, event)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            }
        });

        Self {
            value,
            report,
            status: if read_only {
                "Preferences are read-only; changes apply only to this session.".into()
            } else if persistent {
                "Preferences loaded.".into()
            } else {
                "Session preferences.".into()
            },
            persistent,
            read_only,
            load_warning,
            sender: Some(sender),
            receiver,
            worker: Some(worker),
            generation: 0,
            acknowledged: 0,
        }
    }
    pub fn changed(&mut self) {
        self.value.recovery_interval_seconds = self.value.recovery_interval_seconds.clamp(15, 3600);
        self.generation = self.generation.saturating_add(1);
        if self.read_only {
            self.acknowledged = self.generation;
            self.status = "Preferences are read-only; changes apply only to this session.".into();
            return;
        }
        self.status = if self.persistent {
            "Saving preferences…".into()
        } else {
            "Applying session preferences…".into()
        };
        if self
            .sender
            .as_ref()
            .is_none_or(|tx| tx.send((self.generation, self.value.clone())).is_err())
        {
            self.status = "Preferences worker unavailable; persistence is unconfirmed.".into();
        }
    }
    pub fn poll(&mut self) {
        while let Ok((generation, result)) = self.receiver.try_recv() {
            match result {
                PreferenceWrite::Saved(message) => {
                    self.acknowledged = self.acknowledged.max(generation);
                    if generation == self.generation {
                        self.status = if self.persistent {
                            message
                        } else {
                            "Session preferences applied.".into()
                        };
                    }
                }
                PreferenceWrite::Failed(error) => {
                    if generation == self.generation {
                        self.status = format!("Preferences save failed: {error}");
                    }
                }
            }
        }
    }
    pub fn pending(&self) -> bool {
        self.acknowledged < self.generation
    }
}
impl Drop for Preferences {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("MBSE-Craft preferences worker ended unexpectedly");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn area() -> PathBuf {
        let root = std::env::temp_dir();
        assert!(
            root.starts_with("/workspace"),
            "M1 settings tests require TMPDIR on verified /workspace"
        );
        let path = root.join(format!(
            "mbse-settings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn defaults_off_and_actual_store_debounces_while_preserving_unknowns() {
        assert!(!RecoveryPreferences::default().periodic_recovery);
        let root = area();
        let file = root.join("config.toml");
        std::fs::write(&file,"schema_version = 1\n# retained note\nperiodic_recovery = false\nrecovery_interval_seconds = 60\nfuture_key = 'preserved'\n").unwrap();
        let (mut store, report) =
            SettingsStore::<RecoveryPreferences>::open(Location::Dir(root.clone()));
        assert!(!report.has_problems());
        let now = Instant::now();
        store.update_at(now, |p| p.periodic_recovery = true);
        assert!(matches!(
            store.tick(now + Duration::from_millis(499)).status,
            craft_settings::SaveStatus::Pending
        ));
        assert!(std::fs::read_to_string(&file)
            .unwrap()
            .contains("periodic_recovery = false"));
        assert!(matches!(
            store.tick(now + Duration::from_millis(500)).status,
            craft_settings::SaveStatus::Saved
        ));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            text.contains("# retained note")
                && text.contains("future_key = 'preserved'")
                && text.contains("periodic_recovery = true")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn newer_schema_stays_byte_exact_and_read_only() {
        let root = area();
        let file = root.join("config.toml");
        let bytes =
            "schema_version = 99\nperiodic_recovery = true\nrecovery_interval_seconds = 15\n";
        std::fs::write(&file, bytes).unwrap();
        let (mut store, report) =
            SettingsStore::<RecoveryPreferences>::open(Location::Dir(root.clone()));
        assert!(report.read_only.is_some());
        store.update(|p| p.periodic_recovery = false);
        assert!(store.flush().is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn actual_store_failure_reports_backoff_and_retries() {
        let root = area();
        let (mut store, _) =
            SettingsStore::<RecoveryPreferences>::open(Location::Dir(root.clone()));
        let now = Instant::now();
        store.update_at(now, |p| p.periodic_recovery = true);
        std::fs::remove_dir(&root).unwrap();
        std::fs::write(&root, b"owned failure fixture").unwrap();
        let failed = store.tick(now + Duration::from_secs(1));
        assert!(matches!(
            failed.status,
            craft_settings::SaveStatus::Failed(_)
        ));
        let retry = failed.next_deadline.unwrap();
        assert!(retry > now + Duration::from_secs(1));
        assert!(matches!(
            store.tick(retry - Duration::from_millis(1)).status,
            craft_settings::SaveStatus::Pending
        ));
        std::fs::remove_file(&root).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert!(matches!(
            store.tick(retry).status,
            craft_settings::SaveStatus::Saved
        ));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod worker_adapter_tests {
    use super::*;
    fn area() -> tempfile::TempDir {
        assert!(std::env::temp_dir().starts_with("/workspace"));
        tempfile::tempdir().unwrap()
    }
    fn until(p: &mut Preferences, check: impl Fn(&Preferences) -> bool) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            p.poll();
            if check(p) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }
    #[test]
    fn actual_worker_retries_same_generation_after_local_write_failure() {
        let area = area();
        let config = area.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let mut prefs = Preferences::open(Some(config.clone()), eframe::egui::Context::default());
        prefs.value.periodic_recovery = true;
        prefs.changed();
        assert!(until(&mut prefs, |p| !p.pending()));
        let held = area.path().join("config-held");
        std::fs::rename(&config, &held).unwrap();
        std::fs::write(&config, b"owned test blocker").unwrap();
        prefs.value.recovery_interval_seconds = 30;
        prefs.changed();
        let generation = prefs.generation;
        let failure_seen = until(&mut prefs, |p| p.status.contains("save failed"));
        let remained_pending = prefs.pending();
        std::fs::remove_file(&config).unwrap();
        std::fs::rename(&held, &config).unwrap();
        // No changed() call after repair: retry must belong to the same generation.
        let retried = until(&mut prefs, |p| !p.pending());
        assert!(failure_seen && remained_pending && retried);
        assert_eq!(prefs.generation, generation);
        assert_eq!(prefs.acknowledged, generation);
        assert!(std::fs::read_to_string(config.join("config.toml"))
            .unwrap()
            .contains("recovery_interval_seconds = 30"));
    }
    #[test]
    fn older_saved_generation_cannot_clear_latest_pending_worker_change() {
        let area = area();
        let config = area.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let mut prefs = Preferences::open(Some(config.clone()), eframe::egui::Context::default());
        prefs.value.periodic_recovery = true;
        prefs.changed();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while !config.join("config.toml").is_file() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(config.join("config.toml").is_file());
        prefs.value.recovery_interval_seconds = 42;
        prefs.changed();
        assert!(until(&mut prefs, |p| p.acknowledged >= 1));
        assert!(prefs.pending());
        assert!(prefs.acknowledged < prefs.generation);
        assert!(!prefs.status.starts_with("Preferences saved"));
        assert!(until(&mut prefs, |p| !p.pending()));
        assert!(std::fs::read_to_string(config.join("config.toml"))
            .unwrap()
            .contains("recovery_interval_seconds = 42"));
    }
}

#[cfg(test)]
mod shortcut_persistence_tests {
    use super::*;
    #[test]
    fn additive_overrides_round_trip_with_existing_recovery_values_comments_and_unknown_keys(){
        let area=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let path=area.path().join("config.toml");std::fs::write(&path,"schema_version = 1\n# retained comment\nperiodic_recovery = true\nrecovery_interval_seconds = 42\nfuture_key = 'retained'\n[shortcut_overrides]\n'future.command' = 'future syntax retained'\n").unwrap();
        let(mut store,report)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert!(!report.has_problems());store.update(|p|{p.shortcut_overrides.insert("edit.undo".into(),"Primary+Alt+U".into());});store.flush().unwrap();
        let text=std::fs::read_to_string(&path).unwrap();assert!(text.contains("# retained comment")&&text.contains("future_key = 'retained'")&&text.contains("future syntax retained"));
        let(reopened,report)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert!(!report.has_problems());assert!(reopened.get().periodic_recovery);assert_eq!(reopened.get().recovery_interval_seconds,42);assert_eq!(reopened.get().shortcut_overrides["edit.undo"],"Primary+Alt+U");assert_eq!(reopened.get().shortcut_overrides["future.command"],"future syntax retained");
    }
    #[test]
    fn actual_worker_shortcut_failure_remains_pending_and_retries_same_generation(){
        let area=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let config=area.path().join("config");std::fs::create_dir(&config).unwrap();let mut p=Preferences::open(Some(config.clone()),eframe::egui::Context::default());
        fn until(p:&mut Preferences,check:impl Fn(&Preferences)->bool)->bool{let end=Instant::now()+Duration::from_secs(8);while Instant::now()<end{p.poll();if check(p){return true;}std::thread::sleep(Duration::from_millis(20));}false}
        p.value.shortcut_overrides.insert("edit.undo".into(),"Primary+Alt+U".into());p.changed();assert!(until(&mut p,|p|!p.pending()));
        let held=area.path().join("held-config");std::fs::rename(&config,&held).unwrap();std::fs::write(&config,b"owned P10 failure fixture").unwrap();p.value.shortcut_overrides.insert("view.layout_undo".into(),"Primary+Alt+L".into());p.changed();let generation=p.generation;let failure=until(&mut p,|p|p.status.contains("save failed"));let pending=p.pending();std::fs::remove_file(&config).unwrap();std::fs::rename(&held,&config).unwrap();assert!(failure&&pending);assert!(until(&mut p,|p|!p.pending()));assert_eq!(p.generation,generation);assert_eq!(p.acknowledged,generation);let text=std::fs::read_to_string(config.join("config.toml")).unwrap();assert!(text.contains("Primary+Alt+L")&&text.contains("Primary+Alt+U"));
    }
}

#[cfg(test)]
mod shortcut_reset_test {
    use super::*;
    #[test]
    fn explicit_default_marker_survives_merge_save_and_restart_without_resurrecting_old_override(){
        let area=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let path=area.path().join("config.toml");std::fs::write(&path,"schema_version = 1\n# keep reset note\n[shortcut_overrides]\n'edit.undo' = 'Primary+Alt+U'\n'future.command' = 'keep'\n").unwrap();
        let(mut store,_)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));let mut draft=crate::shortcuts::Editor::new(&store.get().shortcut_overrides);draft.reset_known();store.update(|p|p.shortcut_overrides=draft.draft);store.flush().unwrap();
        let(store,_)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert_eq!(store.get().shortcut_overrides["edit.undo"],"Default");assert_eq!(store.get().shortcut_overrides["future.command"],"keep");let active=crate::shortcuts::Bindings::resolve(&store.get().shortcut_overrides).unwrap_or_else(|e|panic!("{e:?}"));assert_eq!(active.get(crate::command_catalog::Id::Undo),Some(crate::command_catalog::Id::Undo.spec().shortcut));let text=std::fs::read_to_string(path).unwrap();assert!(text.contains("# keep reset note")&&!text.contains("Primary+Alt+U"));
    }
}

#[cfg(test)]mod workspace_persistence_tests{
 use super::*;use crate::{docking::{Panel,WorkspaceDock},perspectives};use craft_ui::docking::Action;
 #[test]fn actual_store_reopen_perspective_and_explicit_reset_preserve_other_preferences(){
  let area=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let path=area.path().join("config.toml");std::fs::write(&path,"schema_version = 1\n# workspace note\nperiodic_recovery = true\nrecovery_interval_seconds = 42\nfuture_key = 'retained'\n[shortcut_overrides]\n'edit.undo' = 'Primary+Alt+U'\n").unwrap();
  let mut d=WorkspaceDock::default();d.apply(0,vec![Action::Float{panel:Panel::Inspector,rect:[90.,160.,500.,700.]}]).unwrap();let text=perspectives::encode(&d.layout).unwrap();
  {let(mut store,report)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert!(!report.has_problems());store.update(|p|{p.workspace_layout=text.clone();p.workspace_perspectives.insert("Review".into(),text.clone());});store.flush().unwrap();}
  {let(mut store,report)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert!(!report.has_problems());assert_eq!(perspectives::decode(&store.get().workspace_layout).unwrap(),d.layout);assert_eq!(store.get().workspace_perspectives["Review"],text);store.update(|p|p.workspace_layout=String::new());store.flush().unwrap();}
  let(store,report)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert!(!report.has_problems());assert!(store.get().workspace_layout.is_empty());assert_eq!(store.get().workspace_perspectives["Review"],text);assert!(store.get().periodic_recovery);assert_eq!(store.get().recovery_interval_seconds,42);assert_eq!(store.get().shortcut_overrides["edit.undo"],"Primary+Alt+U");let bytes=std::fs::read_to_string(path).unwrap();assert!(bytes.contains("# workspace note")&&bytes.contains("future_key = 'retained'"));
 }
 #[test]fn invalid_workspace_payload_retains_original_bytes_and_other_valid_settings(){
  let area=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let path=area.path().join("config.toml");let bytes="schema_version = 1\nworkspace_layout = '{broken'\nperiodic_recovery = true\nrecovery_interval_seconds = 42\n";std::fs::write(&path,bytes).unwrap();let(store,report)=SettingsStore::<RecoveryPreferences>::open(Location::Dir(area.path().into()));assert!(!report.has_problems());assert!(perspectives::decode(&store.get().workspace_layout).is_err());assert!(store.get().periodic_recovery);assert_eq!(std::fs::read_to_string(path).unwrap(),bytes);
 }
}

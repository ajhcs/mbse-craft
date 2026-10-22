//! App-owned opt-in policy; persistence is the audited upstream craft-settings store.
use craft_settings::{Location, Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecoveryPreferences {
    pub periodic_recovery: bool,
    pub recovery_interval_seconds: u64,
}
impl Default for RecoveryPreferences {
    fn default() -> Self {
        Self {
            periodic_recovery: false,
            recovery_interval_seconds: 60,
        }
    }
}
impl Settings for RecoveryPreferences {
    const APP_ID: &'static str = "mbse-craft";
    const SCHEMA_VERSION: u32 = 1;
}

pub struct Preferences {
    pub value: RecoveryPreferences,
    pub report: String,
    pub status: String,
    pub persistent: bool,
    pub read_only: bool,
    sender: Option<Sender<(u64, RecoveryPreferences)>>,
    receiver: Receiver<(u64, Result<String, String>)>,
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
            while let Ok((mut generation, mut value)) = updates.recv() {
                for next in updates.try_iter() {
                    generation = next.0;
                    value = next.1;
                }
                store.update(|prefs| *prefs = value);
                let result = store
                    .flush()
                    .map(|_| {
                        let moved = store.take_moved_aside();
                        if moved.is_empty() {
                            "Preferences saved.".into()
                        } else {
                            format!("Preferences saved; damaged file retained at {moved:?}")
                        }
                    })
                    .map_err(|e| e.to_string());
                if results.send((generation, result)).is_err() {
                    break;
                }
                ctx.request_repaint();
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
            self.acknowledged = self.generation;
        }
    }
    pub fn poll(&mut self) {
        while let Ok((generation, result)) = self.receiver.try_recv() {
            self.acknowledged = self.acknowledged.max(generation);
            if generation == self.generation {
                self.status = match result {
                    Ok(message) => {
                        if self.persistent {
                            message
                        } else {
                            "Session preferences applied.".into()
                        }
                    }
                    Err(error) => format!("Preferences save failed: {error}"),
                };
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

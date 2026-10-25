mod command_catalog;
mod assurance_ui;
mod catalog_ui;
mod exchange_ui;
mod semantic_ui;
mod source_edit_ui;
mod preferences;
mod periodic_recovery;
mod ui;
mod views_ui;
use mbse_model::{stage1::RecordId, stage1_io::Session};
use std::path::PathBuf;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut state = None;
    let mut config = None;
    let mut open = None;
    let mut fixture = None;
    let mut recover = None;
    let mut capture = None;
    let mut selected = None;
    while let Some(flag) = args.next() {
        if flag == "--help" {
            println!("MBSE-Craft native model editor\nRequired: --state-dir DIR (owned recovery directory)\nOptional: --config-dir DIR (separate owned config directory; otherwise session-only preferences)\n  --open FILE | --fixture JSON | --recover FILE\n  --select ID --capture NEW.png\nNo network or external execution. Restricted SysML copies via the UI.");
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value after {flag}"))?;
        match flag.as_str() {
            "--state-dir" => state = Some(PathBuf::from(value)),
            "--config-dir" => config = Some(PathBuf::from(value)),
            "--open" => open = Some(PathBuf::from(value)),
            "--fixture" => fixture = Some(PathBuf::from(value)),
            "--recover" => recover = Some(PathBuf::from(value)),
            "--capture" => capture = Some(PathBuf::from(value)),
            "--select" => {
                selected = Some(RecordId(value.parse().map_err(|_| "Invalid record ID")?))
            }
            _ => return Err(format!("Unknown option: {flag}")),
        }
    }
    let state = state.ok_or("Use --state-dir with an owned directory on /workspace")?;
    if [open.is_some(), fixture.is_some(), recover.is_some()]
        .into_iter()
        .filter(|v| *v)
        .count()
        > 1
    {
        return Err("Choose one of --open, --fixture or --recover".into());
    }
    let mut session = if let Some(path) = open {
        Session::open(&state, &path)
    } else if let Some(path) = recover {
        Session::restore(&state, &path)
    } else if let Some(path) = fixture {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Session::from_fixture(&state, &value)
    } else {
        Session::new(&state)
    }?;
    let config = config
        .map(|p| validate_config_dir(&p, &session.state_root))
        .transpose()?;
    if let Some(id) = selected {
        session.editor.select(id)?;
    }
    let frame_choice = craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT"));
    eprintln!(
        "MBSE-Craft frame policy: {:?}; override {:?}",
        frame_choice.frame(),
        frame_choice.overridden()
    );
    let viewport = craft_window::decorate(
        eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440., 1000.])
            .with_min_inner_size([1000., 700.]),
        frame_choice.frame(),
    );
    eframe::run_native(
        "MBSE-Craft · Model workspace",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(ui::Workspace::new(
                session,
                capture,
                config,
                frame_choice,
                &cc.egui_ctx,
            )))
        }),
    )
    .map_err(|e| e.to_string())
}
fn validate_config_dir(path: &std::path::Path, state: &std::path::Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || !path.starts_with("/workspace")
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(
            "--config-dir must be an absolute owned /workspace directory without parent traversal"
                .into(),
        );
    }
    let mut ancestor = path;
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or("Config directory has no existing parent")?;
    }
    let canonical = std::fs::canonicalize(ancestor).map_err(|e| e.to_string())?;
    if !canonical.starts_with("/workspace") {
        return Err("Config directory resolves outside /workspace".into());
    }
    let suffix = path.strip_prefix(ancestor).map_err(|e| e.to_string())?;
    let resolved = canonical.join(suffix);
    if resolved.starts_with(state) || state.starts_with(&resolved) {
        return Err(
            "--config-dir and --state-dir must use separate non-overlapping directories".into(),
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if std::fs::metadata(ancestor)
            .map_err(|e| e.to_string())?
            .uid()
            != std::fs::metadata(state).map_err(|e| e.to_string())?.uid()
        {
            return Err("Config parent must have the same owner as the app data directory".into());
        }
    }
    if path.exists()
        && std::fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("Config directory must not be a symlink".into());
    }
    Ok(resolved)
}
fn main() {
    if let Err(error) = run() {
        eprintln!("MBSE-Craft: {error}");
        std::process::exit(1);
    }
}

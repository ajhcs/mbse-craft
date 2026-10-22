mod ui;
use mbse_model::{stage1::RecordId, stage1_io::Session};
use std::path::PathBuf;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut state = None;
    let mut open = None;
    let mut fixture = None;
    let mut recover = None;
    let mut capture = None;
    let mut selected = None;
    while let Some(flag) = args.next() {
        if flag == "--help" {
            println!("MBSE-Craft native model editor\nRequired: --state-dir DIR (owned recovery directory)\nOptional: --open FILE | --fixture JSON | --recover FILE\n  --select ID --capture NEW.png\nNo network or external execution. Restricted SysML copies via the UI.");
            return Ok(());
        }
        let value = args.next().ok_or_else(|| format!("Missing value after {flag}"))?;
        match flag.as_str() {
            "--state-dir" => state = Some(PathBuf::from(value)),
            "--open" => open = Some(PathBuf::from(value)),
            "--fixture" => fixture = Some(PathBuf::from(value)),
            "--recover" => recover = Some(PathBuf::from(value)),
            "--capture" => capture = Some(PathBuf::from(value)),
            "--select" => selected = Some(RecordId(value.parse().map_err(|_| "Invalid record ID")?)),
            _ => return Err(format!("Unknown option: {flag}")),
        }
    }
    let state = state.ok_or("Use --state-dir with an owned directory on /workspace")?;
    if [open.is_some(), fixture.is_some(), recover.is_some()].into_iter().filter(|v| *v).count() > 1 {
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
    if let Some(id) = selected { session.editor.select(id)?; }
    eframe::run_native("MBSE-Craft · Model workspace", eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1440., 1000.]).with_min_inner_size([1000., 700.]),
        ..Default::default()
    }, Box::new(move |cc| Ok(Box::new(ui::Workspace::new(session, capture, &cc.egui_ctx)))))
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = run() { eprintln!("MBSE-Craft: {error}"); std::process::exit(1); }
}

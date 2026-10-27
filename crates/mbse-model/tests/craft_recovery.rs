//! MBSE-Craft integration oracle: corrupt main bytes, recover exact retained work.
use mbse_model::{
    stage1::{Command, RawKey, RecordId},
    stage1_io::{execute_write, Session},
};
#[test]
fn actual_craft_save_and_recovery_survive_corrupt_main() {
    let root = std::env::var_os("TMPDIR").expect("explicit workspace TMPDIR");
    assert!(std::path::Path::new(&root).starts_with("/workspace"));
    let area = tempfile::tempdir_in(root).unwrap();
    let state = area.path().join("state");
    let native = area.path().join("model.kernspan");
    let mut session = Session::new(&state).unwrap();
    session
        .editor
        .command(
            &session.editor.freshness(),
            Command::EditText {
                key: RawKey {
                    record: RecordId(1),
                    field: "purpose".into(),
                    item: None,
                },
                text: "Retained reviewable purpose".into(),
            },
        )
        .unwrap();
    let save = session.begin_save(&native).unwrap();
    let saved = execute_write(save);
    assert!(saved.succeeded());
    assert!(session.complete(saved));
    let recovery = session.begin_recovery().unwrap();
    let recovery_path = recovery.ticket.destination.clone();
    let result = execute_write(recovery);
    assert!(result.succeeded());
    assert!(session.complete(result));
    std::fs::write(&native, b"{truncated").unwrap();
    assert!(Session::open(&state, &native).is_err());
    let restored = Session::restore(&state, &recovery_path).unwrap();
    assert_eq!(
        restored.editor.snapshot.records[&RecordId(1)].text("purpose"),
        "Retained reviewable purpose"
    );
    assert!(restored.is_dirty());
    assert!(restored.path.is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&recovery_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
}

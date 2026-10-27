# P12 persisted workspace arrangements and perspectives

Build on merged P11. Enable the already vendored Craft docking serde feature,
with no new downloaded package or vendor modification. An app-owned versioned
JSON string in additive schema-1 preferences stores only panel arrangement.
Decode at most 16 KiB, validate the exact three-panel registry, then activate;
malformed, duplicate/missing/unknown panels and future versions retain original
settings bytes and use defaults. Native model, native views and raw drafts are
never serialized here. The existing settings worker owns debounce, exact generation
acknowledgments, errors/retries and future-schema read-only/session semantics.

Panel moves persist the latest arrangement. Named perspectives (eight names,
1..64 bytes each) store validated snapshots. Reset explicitly persists the current
default marker without deleting named perspectives. Restore/reset must pass the
same native-write/gesture/modal guard as docking and preserve all native content.
Floating rectangles are clamped into the currently available workspace after
startup and viewport resize; no off-screen header is required to recover a panel.
Invalid viewport geometry is refused. Saved source arrangement remains recoverable
as a named perspective. P11's individual-tab versus whole-group scope remains.

Acceptance uses actual store reopen, reset and corruption tests, retained native
raw9e, named save/restore and cold GUI launches loading exact previously saved
config copies inside owned run directories. This does not claim same physical
config path or native OS floating windows, independent review or full parity.

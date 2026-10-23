# Active-view camera controls

Status: implementation in progress, unrun. This isolated continuation starts at qualified scoped search643f97b under the existing owner. The Samsung workspace was freshly verified read-write. Previous qualified worktrees and evidence stay frozen; no publication is authorized here.

This is the next bounded M2B CAT-011/CAP-019 path. The existing native views v1 Camera and Layout::Camera already carry per-view x/y/zoom, history and Save authority, and the renderer already applies them. The UI adds 25-percent zoom steps, 100-percent zoom, 64-screen-pixel viewport pan steps and Reset camera. Zoom keeps the view origin fixed. Pan divides the screen step by current zoom. Cameras remain independent per view; occurrence geometry and native meaning do not change.

Camera::checked centralizes the existing finite coordinate and0.05–20zoom limits before controller dispatch; document validation uses the same rule. No profile/field changes or new checkpoint authority. No-op and rejected commands do not consume layout revision/history or discard redo. Active occurrence gestures block camera controls, Fit, layout Undo/Redo and tab switches. Existing normal native Open and explicit Open views remain separate admissions. Ephemeral widget rectangles and active-camera audit data support verification without creating presentation state.

Focused model/controller tests, independent source review, one offline test/build gate and an actual native per-view zoom/pan/history/Save/reopen workflow remain required. No renderer range/performance/accessibility/platform or full feature-parity acceptance is implied.

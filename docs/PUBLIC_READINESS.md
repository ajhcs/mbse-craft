# Contributor and publication handoff

Prepared 2026-10-27 from merged integration `2858c353ce2277a76d41b89e29591bb7903e276b` (PR #62). This is a source-contribution checkpoint, not a software release or full parity acceptance. The repository remains private. No visibility change, invitation, release, deployment, history rewrite or worktree removal is part of this preparation.

## Work ready for contribution

P01–P39 are merged into `implementation/native-alpha`, including the split children. Nineteen roadmap positions remain, P40–P58; future PR count can exceed nineteen. The canonical 58-position plan is unchanged. All 56 broad acceptance specifications and integrated milestone exits remain open. Current behavior and source-specific qualification are recorded by the individual PR receipts, not by a summed historical test count.

PR #62 completes bounded native source import review/Apply/Undo/edit/save/cold reopen. Its [isolated review](parity/p39c/REVIEW.md) retains all failures and exact scope: full 835-test/build evidence on the corrected source, followed by a display-only final source rebuild and cold native checks. There is no final-source full-suite claim. The work does not establish vendor-created No Magic interoperability or full SysML semantics.

[CONTRIBUTING.md](../CONTRIBUTING.md) gives the actual branch, toolchain, isolated-worktree and acceptance workflow. The current default `main` is still a skeleton. Existing PR #1 proposes promoting the integrated branch to `main`; its refreshed description identifies the final prepared promotion snapshot and its evidence boundaries. Preserve the integration branch and local canonical checkout during that remote merge. No branch deletion or force push is needed. After promotion, update the contributor entry point to `main` in a small follow-up.

## Publication scope reviewed

Changing this existing repository to public exposes its retained Git history, branches, PR discussions and evidence, not only the current README. No sanitizing rewrite was performed. Recorded evidence intentionally includes local user/home and workspace paths, device identifiers, process/job details and worktree/branch inventories. For example, `docs/parity/p02/verification-local-20261025/REGRESSION_RECEIPT.json` retains the workspace device serial/UUID; `docs/GUIDANCE_PROVENANCE.json` retains local provenance paths. At the merged PR #62 tree, literal checks found `/home/` in 323 tracked files, `/workspace/` in 1301, the known workspace UUID in 255 and its device serial in 252. These counts cover uncompressed tracked bytes only; compressed receipts and older history can contain further copies.

The original MIT app license, retained Kernspan MIT attribution, syntax Apache license, pinned Craft/egui/winit licenses and font notices, and dependency notices remain in the tree. [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) and the upstream SOURCE/provenance manifests identify the adopted material. Restricted ArtCraft services, unlicensed shared workflows and proprietary CATIA assets are not adopted by this work. This is a source/notices review, not a new legal or binary-distribution qualification.

## Secret scan evidence

Installed gitleaks 8.30.1 completed two read-only scans with `--redact=100 --ignore-gitleaks-allow`, no custom configuration, exit 0 and zero findings. The history command used `git --log-opts=--all` after fetching origin. It covers locally reachable refs; it does not establish completeness of hidden provider refs, private issues or external storage. The snapshot scan used a clean `git archive` of P39c candidate `7bab104b657eaa9edeafeac06e276b371ba77e65`, `dir --max-archive-depth=2`. The later P39c review commits add review documentation only. Both redacted reports are retained in [public-readiness](public-readiness); each is the three-byte empty JSON array, SHA-256 `37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570`. A third zero-finding scan covers the four new or changed handoff Markdown documents before review; its empty report is retained as `new-docs-redacted.json`. A secret scanner is bounded evidence, not a guarantee that every sensitive fact has been identified.

## Final operator decision

The concrete publication choice is to retain the existing auditable history, including the machine metadata above, promote PR #1 containing the reviewed integration to the default `main`, and change `ajhcs/mbse-craft` from private to public. Publication makes those contents available for copying. The operator asked to be notified when ready; visibility remains unchanged until explicit go-ahead. If the retained metadata should stay private, choose a separately reviewed publication scope before changing visibility; do not silently rewrite this history.

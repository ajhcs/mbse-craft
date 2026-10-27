# Contributor and publication handoff

Prepared 2026-10-27 from merged integration `2858c353ce2277a76d41b89e29591bb7903e276b` (PR #62). This is a source-contribution checkpoint, not a software release or full parity acceptance. The original preparation below has been followed by the operator-approved merge/publication. On 2026-10-27, PR #1 merged as `de65951c24f3a8d7db779d75fa5c0ee3d076d5e0`; `main` became the implemented contribution base and GitHub visibility was confirmed PUBLIC. The complete reviewed integration is an ancestor and its tree matches the promotion merge. History and existing branches/worktrees remain retained. No invitation, release, deployment, history rewrite or worktree removal occurred.

## Work ready for contribution

P01–P39 are merged into `implementation/native-alpha`, including the split children. Nineteen roadmap positions remain, P40–P58; future PR count can exceed nineteen. The canonical 58-position plan is unchanged. All 56 broad acceptance specifications and integrated milestone exits remain open. Current behavior and source-specific qualification are recorded by the individual PR receipts, not by a summed historical test count.

PR #62 completes bounded native source import review/Apply/Undo/edit/save/cold reopen. Its [isolated review](parity/p39c/REVIEW.md) retains all failures and exact scope: full 835-test/build evidence on the corrected source, followed by a display-only final source rebuild and cold native checks. There is no final-source full-suite claim. The work does not establish vendor-created No Magic interoperability or full SysML semantics.

[CONTRIBUTING.md](../CONTRIBUTING.md) gives the actual branch, toolchain, isolated-worktree and acceptance workflow. PR #1 promoted the integrated branch to default `main`. Its description records the source-specific evidence and limits. The contribution instructions now use `main`. The prior integration branch and canonical local checkout remain preserved; no branch deletion or force push was needed.

## Publication scope reviewed

Changing this existing repository to public exposes its retained Git history, branches, PR discussions and evidence, not only the current README. No sanitizing rewrite was performed. Recorded evidence intentionally includes local user/home and workspace paths, device identifiers, process/job details and worktree/branch inventories. For example, `docs/parity/p02/verification-local-20261025/REGRESSION_RECEIPT.json` retains the workspace device serial/UUID; `docs/GUIDANCE_PROVENANCE.json` retains local provenance paths. At the merged PR #62 tree, literal checks found `/home/` in 323 tracked files, `/workspace/` in 1301, the known workspace UUID in 255 and its device serial in 252. These counts cover uncompressed tracked bytes only; compressed receipts and older history can contain further copies.

The original MIT app license, retained Kernspan MIT attribution, syntax Apache license, pinned Craft/egui/winit licenses and font notices, and dependency notices remain in the tree. [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) and the upstream SOURCE/provenance manifests identify the adopted material. Restricted ArtCraft services, unlicensed shared workflows and proprietary CATIA assets are not adopted by this work. This is a source/notices review, not a new legal or binary-distribution qualification.

## Secret scan evidence

Installed gitleaks 8.30.1 completed two read-only scans with `--redact=100 --ignore-gitleaks-allow`, no custom configuration, exit 0 and zero findings. The history command used `git --log-opts=--all` after fetching origin. It covers locally reachable refs; it does not establish completeness of hidden provider refs, private issues or external storage. The snapshot scan used a clean `git archive` of P39c candidate `7bab104b657eaa9edeafeac06e276b371ba77e65`, `dir --max-archive-depth=2`. The later P39c review commits add review documentation only. Both redacted reports are retained in [public-readiness](public-readiness); each is the three-byte empty JSON array, SHA-256 `37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570`. A third zero-finding scan covers the four new or changed handoff Markdown documents before review; its empty report is retained as `new-docs-redacted.json`. A secret scanner is bounded evidence, not a guarantee that every sensitive fact has been identified.

## Recorded operator decision

The operator explicitly approved merging PR #1 into `main` and making `ajhcs/mbse-craft` public with the history and all work retained. Both actions completed. Publication exposes the retained history and metadata described above. The approval did not authorize history rewriting or deleting branches/worktrees. See the [branch integration audit](BRANCH_INTEGRATION.md) for the remaining historical lane/proposal references.

# Main integration and contributor consolidation

PR #1 merged on 2026-10-27 as `de65951c24f3a8d7db779d75fa5c0ee3d076d5e0`; its tree exactly matches reviewed integration `614475c2967b27dba890ba260aa641442b3a789e`, which remains an ancestor. GitHub reports PUBLIC visibility and default `main`. The canonical local checkout remains at its original frozen head; remote `main` is the base for new work.

## Branch audit

The [frozen audit](public-readiness/BRANCH_AUDIT.json) covers 234 locally registered branch refs at the promoted main: 169 local and65 origin refs. Of these,207 refs are ancestors of main. All64 published implementation/feature/review/main branch refs are already contained; the one other published ref is the old recovery-copy proposal. Twenty-seven refs (26 distinct heads) are not ancestors, largely old lanes whose changes were cherry-picked. A non-ancestor ref alone does not mean product behavior is missing.

`git cherry` narrowed those lanes to five distinct patches requiring source inspection. Three have current equivalents: native anchor binding plus its named regression; assurance runtime-proof/historical-review documentation and implementation; and the exact-toolchain build script (byte-identical, with later README improvements). The other two are historical recovery-copy planning documents. Their four files are now retained byte-for-byte under [history/recovery-copy-proposal](history/recovery-copy-proposal/README.md), with original commit/path/byte/SHA identities. P04/P07/P08 provide the later implementation and exact evidence. No additional feature merge or Rust change was required.

This audit is repository-ref and source inspection, not a new app regression run. It does not assert completeness of hidden provider refs or outside work. It preserves the distinction between historical plans and accepted implementation.

## Contributor entry points

Use [README](../README.md) and [CONTRIBUTING](../CONTRIBUTING.md) on `main`. The front page now points to the current roadmap, focused first tasks, actual build constraints and source-specific evidence. Its former long epoch narrative is preserved as a [historical page](history/README-before-main-promotion.md), with relative links adjusted. Build helpers, schemas, model authority and qualification receipts are unchanged.

Before this consolidation, all118 available registered checkouts were clean at their recorded heads;18 user-removed directories remain registered and are neither recreated nor pruned. No worktree or branch deletion, force push, squash or historical rewrite is part of this change. Existing branches remain available for provenance. A future branch-retirement batch needs an exact candidate list and separate approval.

# Pinned craft-libs extraction

Source: https://github.com/storytold/craft-libs/tree/b1401e33b105ca2861575ede90f67f4d54449001

Unmodified atomic, UI, settings, quit, docpath, window and platform crate directories, with full upstream LICENSE-MIT, LICENSE-APACHE, NOTICE and ATTRIBUTION.md. `SOURCE.json` records SHA-256 of every retained input. Only the extraction workspace manifest is new, replacing the upstream workspace with these seven members and an exact egui 0.36.2 pin. None of the seven selected crates has a build script. No service, PDF backend, font asset, branded artwork or upstream automation is included. All paths in the original upstream README/NOTICE may refer to excluded upstream material.

The application must exclude this nested workspace from its own workspace (exclude the nested vendor workspace and selected crate directories as required by Cargo workspace discovery) and use path dependencies to selected `vendor/craft-libs/crates/<name>`. Production application calls, document semantics and acceptance belong to MBSE-Craft. This extraction does not imply ArtCraft file, service or launcher compatibility.

M1 extends the extraction with genuine lifecycle/storage/path/window/platform source contracts. Merely retaining source is not adoption: each app call and conditional feature must be identified and tested by its owning implementation. Platform NOTICE and dual licenses remain alongside that crate. Dependencies and target APIs still require action-time review before builds.

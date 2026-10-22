# Pinned craft-libs extraction

Source: https://github.com/storytold/craft-libs/tree/b1401e33b105ca2861575ede90f67f4d54449001

Unmodified atomic and UI crate directories, with full upstream LICENSE-MIT, LICENSE-APACHE, NOTICE and ATTRIBUTION.md. `SOURCE.json` records SHA-256 of every retained input. Only the extraction workspace manifest is new, replacing the upstream workspace with these two members and an exact egui 0.36.2 pin. Neither selected crate has a build script. No service, PDF backend, font asset, branded artwork or upstream automation is included. All paths in the original upstream README/NOTICE may refer to excluded upstream material.

The application must exclude this nested workspace from its own workspace (`exclude = ["vendor/craft-libs"]`) and use path dependencies `vendor/craft-libs/crates/atomic` and `vendor/craft-libs/crates/ui`. Production application calls, document semantics and acceptance belong to MBSE-Craft. This extraction does not imply ArtCraft file, service or launcher compatibility.

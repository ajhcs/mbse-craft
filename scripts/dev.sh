#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=1
: "${CARGO_TARGET_DIR:?Set dedicated /workspace MBSE-Craft target directory}"
: "${TMPDIR:?Set dedicated /workspace MBSE-Craft temporary directory}"
: "${CARGO_HOME:?Set dedicated /workspace MBSE-Craft Cargo home}"
exec cargo "$@"

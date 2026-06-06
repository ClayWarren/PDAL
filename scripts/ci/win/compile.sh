#!/bin/bash
set -euo pipefail

ninja -v
ninja install
../rust/scripts/check_installed_capi_consumer.sh --prefix "$CONDA_PREFIX"

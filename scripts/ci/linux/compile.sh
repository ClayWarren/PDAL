#!/bin/bash
set -euo pipefail

ninja
ninja install
../rust/scripts/check_installed_capi_consumer.sh .

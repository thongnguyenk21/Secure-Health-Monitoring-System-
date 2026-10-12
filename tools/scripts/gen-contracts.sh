#!/usr/bin/env bash
# Regenerates shared/rust/src/generated.rs from shared/protocols/proto.
#   tools/scripts/gen-contracts.sh          write the generated code
#   tools/scripts/gen-contracts.sh --check  fail if it is out of date (used by CI)
# Both modes also check that every example in shared/protocols/examples encodes.
set -euo pipefail
cd "$(dirname "$0")/../.."
exec cargo run --quiet --package iomt-codegen -- "$@"

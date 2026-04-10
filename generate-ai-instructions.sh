#!/bin/bash
set -o errexit -o pipefail -o nounset
cd "$(dirname "$0")"

exec cargo run --features='ai-instructions' --bin='lyrics-ai-instructions' -- --generate .

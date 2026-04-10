#!/bin/bash
set -o errexit -o pipefail -o nounset
cd "$(dirname "$0")"

exec cargo run --bin='lyrics-ai-instructions' -- --generate .

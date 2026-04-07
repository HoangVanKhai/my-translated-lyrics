#!/bin/bash
set -o errexit -o nounset -o pipefail

project_dir=$(dirname "$0")
bin_name=install-local-lyrics

(
  cd "$project_dir"
  cargo build --bin=$bin_name
)

"$project_dir/target/debug/$bin_name" "$@"

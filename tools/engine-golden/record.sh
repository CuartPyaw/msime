#!/usr/bin/env bash
# Re-record golden outputs. Usage: record.sh <recorder-binary> <work-dir> [scenario-name ...]
# One process per scenario: PersonalNgramStore is a process-lifetime singleton keyed by journal path.
set -euo pipefail
recorder=$1
work=$2
shift 2
here=$(cd "$(dirname "$0")" && pwd)
golden=$here/../../crates/engine/tests/golden
mkdir -p "$golden/expected" "$work"
tmo=$(command -v timeout || command -v gtimeout)
if [ "$#" -gt 0 ]; then
    names=("$@")
else
    names=()
    for f in "$golden"/scenarios/*.json; do names+=("$(basename "$f" .json)"); done
fi
failed=0
for name in "${names[@]}"; do
    if ! "$tmo" 120 "$recorder" scenario "$golden/scenarios/$name.json" "$work" "$golden/expected/$name.json"; then
        echo "FAILED: $name" >&2
        failed=1
    fi
done
exit $failed

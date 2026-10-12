#!/usr/bin/env bash
# T0 canonical dump — cross-build artifact behaviour consistency test
#
# Purpose: produce a canonicalized record of lscli's behaviour for the whole
# fixture suite on THIS build. Two different builds (different OS runners)
# must produce byte-identical canonical dumps; any difference is a T0 failure.
#
# Canonical basis (fixed; do not relax without re-running the full readiness
# checklist — see BETA-READINESS.md §6 E3):
#   1. Content = stdout + stderr merged (2>&1), one file per fixture.
#   2. Carriage returns are stripped (Windows stdout is \n in Rust, but this
#      makes the comparison independent of any text-mode translation).
#   3. The build-identity line is excluded: any line matching
#      /^lscli [0-9]/ (the --version banner) is dropped, because the git
#      commit injected at build time legitimately differs per build.
#   4. Nothing else may be filtered. In particular, error messages, field
#      states, amounts and byte offsets are semantic and MUST match exactly.
#
# Exit codes are checked per fixture: S1,S2a,S2b,S3,S4,S5 must exit 0;
# S6 must exit 1 (malformed input is rejected).

set -u

FIXTURES_DIR="${1:-g3/fixtures}"
OUT_DIR="${2:-/tmp/t0-dump}"
LSCLI="${3:-target/release/lscli}"

MUST_PASS="S1 S2a S2b S3 S4 S5 S7 S8 S9"
MUST_FAIL="S6"

mkdir -p "$OUT_DIR"

fail=0

canonicalize() {
    # strip \r, drop the build-identity line
    tr -d '\r' | sed '/^lscli [0-9][0-9]*\.[0-9]/d'
}

for f in $MUST_PASS; do
    if "$LSCLI" "$FIXTURES_DIR/$f.psbt" > "$OUT_DIR/$f.txt" 2>&1; then
        :
    else
        echo "T0-DUMP ERROR: $f expected exit 0, got $?" >&2
        fail=1
        continue
    fi
    canonicalize < "$OUT_DIR/$f.txt" > "$OUT_DIR/$f.txt.canonical" \
        && mv "$OUT_DIR/$f.txt.canonical" "$OUT_DIR/$f.txt"
done

for f in $MUST_FAIL; do
    if "$LSCLI" "$FIXTURES_DIR/$f.psbt" > "$OUT_DIR/$f.txt" 2>&1; then
        echo "T0-DUMP ERROR: $f expected nonzero exit, got 0" >&2
        fail=1
    fi
    canonicalize < "$OUT_DIR/$f.txt" > "$OUT_DIR/$f.txt.canonical" \
        && mv "$OUT_DIR/$f.txt.canonical" "$OUT_DIR/$f.txt"
done

# index of files with sizes for quick eyeballing
( cd "$OUT_DIR" && wc -c *.txt > INDEX.txt ) || fail=1

if [ "$fail" -ne 0 ]; then
    echo "T0-DUMP FAILED" >&2
    exit 1
fi
echo "T0-DUMP OK: $(ls "$OUT_DIR"/*.txt | wc -l) fixture dumps in $OUT_DIR"

#!/bin/bash
# Usage: qualification/replay-all.sh REPLAY COMPILER OUT
#
# Replays every release in data/ with COMPILER and compares each object with the
# release's recorded interfaces.tsv. Writes OUT/X.Y/report.tsv per release and
# OUT/summary.md, and exits nonzero if any release fails to build or differs.
set -euo pipefail
replay=$(realpath "$1") compiler=$2 out=$3
root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out"
out=$(realpath "$out")
releases=$(cd "$root/data" && ls -d 2.* | sort -V)
printf '%s\n' $releases | xargs -P "$(nproc)" -I{} sh -c \
  '"$1" --release {} --compiler "$2" --out "$3/{}" --root "$4" > "$3/{}.log" 2>&1 || true' \
  _ "$replay" "$compiler" "$out" "$root"
status=0
{
  echo "| Release | identical-bytes | equivalent | accepted | differs | build-failed |"
  echo "| --- | ---: | ---: | ---: | ---: | ---: |"
  for v in $releases; do
    report=$out/$v/report.tsv
    if [ ! -f "$report" ]; then
      echo "| $v | replay failed: $(tail -1 "$out/$v.log") | | | | |"
      status=1
      continue
    fi
    row="| $v"
    for s in identical-bytes equivalent accepted differs build-failed; do
      n=$(tail -n +2 "$report" | cut -f2 | grep -cx "$s" || true)
      row="$row | $n"
      case $s in differs | build-failed) [ "$n" = 0 ] || status=1 ;; esac
    done
    echo "$row |"
  done
} > "$out/summary.md"
cat "$out/summary.md"
exit $status

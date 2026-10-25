#!/usr/bin/env bash
# Refuse to ship proprietary binaries, card images, databases, or secrets.
set -euo pipefail
cd "$(dirname "$0")/.."

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    mapfile -t files < <(git ls-files)
else
    mapfile -t files < <(find . -type f -not -path './.git/*' -not -path './target/*')
fi

bad=0
for f in "${files[@]}"; do
    [ -f "$f" ] || continue
    base="$(basename "$f")"
    case "$f" in
        *.bin|*.BIN|*.mdb|*.MDB|*.accdb|*.ACCDB|*.exe|*.EXE|*.dll|*.DLL|*.sys|*.SYS|*.zip|*.ZIP|*.7z|*.rar|*.iso|*.img|*.tar|*.gz|*.xz|*.bz2|*.SC|*.sc|rfile[0-9]*)
            echo "proprietary/binary extension: $f"; bad=1 ;;
    esac
    case "$base" in
        TIS2000*|Card_writer*|COSIDS*|spssaabdb*|scasbdb*|sdt2sbdb*|vcsaabdb*)
            echo "proprietary filename: $f"; bad=1 ;;
    esac
    size="$(stat -c%s "$f")"
    if [ "$size" -gt 5242880 ]; then
        echo "file larger than 5 MiB: $f ($size bytes)"; bad=1
    fi
    if [ "$f" != "scripts/check-no-proprietary.sh" ]; then
        if grep -IqE 'ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|BEGIN [A-Z ]*PRIVATE KEY|AKIA[0-9A-Z]{16}|xox[baprs]-' "$f"; then
            echo "possible secret: $f"; bad=1
        fi
    fi
done

if [ "$bad" -ne 0 ]; then
    echo "Refusing: remove the files above before committing." >&2
    exit 1
fi
echo "No proprietary files or obvious secrets found."


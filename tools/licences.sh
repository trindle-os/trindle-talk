#!/bin/sh
# Writes the third-party notices for the Rust packages and the models.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$here/trindle-voice/licences/THIRD-PARTY.txt}
cd "$here"
rows=$(mktemp)
seen=$(mktemp)
trap 'rm -f "$rows" "$seen"' EXIT
for manifest in Cargo.toml trindle-voice/Cargo.toml; do
    cargo +1.93 metadata --format-version 1 --offline --locked --manifest-path "$manifest" \
      | jq -r '.packages[] | select(.name != "trindle-talk" and .name != "talk-voice")
               | [.name, .version, (.license // "unknown"), (.manifest_path | rtrimstr("/Cargo.toml")),
                  ((.authors | join(", ")) as $a | if $a == "" then "the " + .name + " authors" else $a end)] | @tsv'
done | sort -u > "$rows"

{
echo "Third-party notices for Talk"
echo
echo "Talk is free software under the GNU General Public License, version 3 or"
echo "any later version. It is built from the packages below, each under its own"
echo "licence, and uses the speech models described at the end."
echo
while IFS='	' read -r name version licence dir authors; do
    echo "================================================================"
    echo "$name $version"
    case "$name" in trindle-common|trindle-voice)
        echo "Licence: GPL-3.0-or-later"
        echo "Part of Trindle itself."
        echo
        continue ;;
    esac
    echo "Licence: $licence"
    found=0
    for f in "$dir"/LICEN[CS]E* "$dir"/COPYING* "$dir"/NOTICE* "$dir"/UNLICENSE* "$dir"/licen[cs]e*; do
        [ -f "$f" ] || continue
        found=1
        sum=$(sha256sum "$f" | cut -d' ' -f1)
        echo
        echo "--- $(basename "$f")"
        first=$(grep "^$sum " "$seen" | head -1 | cut -d' ' -f2- || true)
        if [ -n "$first" ]; then
            echo "Same text as the one printed under: $first"
        else
            echo "$sum $name $version ($(basename "$f"))" >> "$seen"
            cat "$f"
            echo
        fi
    done
    if [ "$found" = 0 ]; then
        echo
        case "$licence" in
            *MIT*)
                echo "--- MIT (the package holds no licence file; the standard text, under its authors' names)"
                echo "Copyright (c) $authors"
                echo
                cat "$here/tools/licence-texts/MIT" ;;
            *Apache-2.0*)
                echo "--- Apache-2.0 (the package holds no licence file; the standard text)"
                cat "$here/tools/licence-texts/Apache-2.0" ;;
            *)
                echo "The licence text could not be found in this package's source. It is declared as: $licence." ;;
        esac
    fi
    echo
done < "$rows"
echo "================================================================"
cat "$here/trindle-voice/licences/NOTICE-models.txt"
echo
echo "--- supertonic-OpenRAIL-M.txt"
cat "$here/trindle-voice/licences/supertonic-OpenRAIL-M.txt"
} > "$out"
echo "$out"

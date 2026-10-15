#!/usr/bin/env bash
# Usage : scripts/reindex.sh fichier.md
# (Ré)écrit l'index en tête du fichier. Ligne 1 = dernière ligne de l'index, puis tableau "Lignes | Section".
# Titres indexés : #, ## (tels quels), ### (préfixe "— "). Ignore les blocs de code.
set -euo pipefail
f="$1"
body=$(mktemp); idx=$(mktemp); trap 'rm -f "$body" "$idx"' EXIT

# Retire l'index existant (lignes 1..N) et la ligne vide qui le suit.
awk 'NR==1 && /^[0-9]+$/ {skip=$1; next}
     skip && NR<=skip {next}
     skip && NR==skip+1 && /^$/ {next}
     {print}' "$f" > "$body"

awk '/^(```|~~~)/ {fence=!fence}
     !fence && /^#{1,3} / {
       n++; ln[n]=NR; match($0,/^#+/); t=substr($0,RLENGTH+2); gsub(/\|/,"\\|",t)
       title[n]=(RLENGTH==3 ? "— " : "") t
     }
     END {
       if (!n) exit
       N=3+n; off=N+1
       print N; print "| Lignes | Section |"; print "|---|---|"
       for (i=1;i<=n;i++) print "| " ln[i]+off "-" (i<n ? ln[i+1]-1 : NR)+off " | " title[i] " |"
       print ""
     }' "$body" > "$idx"

cat "$idx" "$body" > "$f"

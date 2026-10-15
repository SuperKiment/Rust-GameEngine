#!/usr/bin/env bash
# Hook PostToolUse (Edit/Write) : réindexe un .md du périmètre défini dans CLAUDE.md > "Index des documents .md".
f=$(sed -n 's/.*"file_path" *: *"\([^"]*\)".*/\1/p' | head -1)
[[ "$f" == *.md && -f "$f" ]] || exit 0
root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel)}"
rel="${f#"$root"/}"
case "$(basename "$rel")" in CLAUDE.md|INDEX.md|README.md|conventions-docs.md) exit 0 ;; esac
case "$rel" in
  */*) case "$rel" in docs/*|current-task/*|old-tasks/*|games/*/docs/*) ;; *) exit 0 ;; esac ;;
esac
if [[ "$(head -1 "$f")" =~ ^[0-9]+$ ]] || [ "$(wc -l < "$f")" -ge 80 ]; then
  bash "$root/scripts/reindex.sh" "$f"
fi

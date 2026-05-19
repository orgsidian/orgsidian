#!/usr/bin/env bash
# Sync epics.md → GitHub Issues (one issue per Story N.M).
# Idempotent: matches existing issues by exact title "[Story N.M] <title>"
# and updates body + non-status labels in place. Does not modify status:*
# labels on existing issues (manual is authoritative — LD-55).
#
# This is the bootstrap shell version called out in the correct-course step;
# Story 1.16 ships a Rust binary at tools/issues-sync/ that replaces it.

set -euo pipefail

REPO="${REPO:-orgsidian/orgsidian}"
EPICS_FILE="${EPICS_FILE:-_bmad-output/planning-artifacts/epics.md}"
BRANCH_FOR_LINKS="${BRANCH_FOR_LINKS:-main}"
DRY_RUN="${DRY_RUN:-0}"

if ! command -v gh >/dev/null; then
  echo "error: gh CLI not installed" >&2
  exit 2
fi
if [[ ! -f "$EPICS_FILE" ]]; then
  echo "error: epics file not found at $EPICS_FILE" >&2
  exit 2
fi

log() { printf '%s\n' "$*" >&2; }
run() {
  if [[ "$DRY_RUN" == "1" ]]; then
    log "[dry-run] $*"
  else
    "$@"
  fi
}

# ----- Epic → milestone mapping (from epics.md Epic List)
milestone_for_epic() {
  local e="$1"
  if   (( e <= 6  )); then echo "v0.1"
  elif (( e <= 12 )); then echo "v0.5"
  else                     echo "v1.0"
  fi
}

# ----- Label scheme (minimal subset of LD-55; full set lands in Story 1.13)
ensure_label() {
  local name="$1" color="$2" desc="$3"
  if grep -qx "$name" "$LABELS_CACHE" 2>/dev/null; then return 0; fi
  log "  + creating label: $name"
  run gh label create "$name" --color "$color" --description "$desc" --repo "$REPO" >/dev/null
  echo "$name" >> "$LABELS_CACHE"
}

ensure_milestone() {
  local title="$1"
  local existing
  existing=$(jq -r --arg t "$title" '.[] | select(.title == $t) | .number' "$MILESTONES_CACHE" | head -n1)
  if [[ -n "$existing" ]]; then
    echo "$existing"; return 0
  fi
  log "  + creating milestone: $title"
  local num
  if [[ "$DRY_RUN" == "1" ]]; then
    log "[dry-run] gh api repos/$REPO/milestones -f title=$title"
    num="0"
  else
    num=$(gh api "repos/$REPO/milestones" -f title="$title" --jq '.number')
    # refresh cache
    gh api "repos/$REPO/milestones?state=all&per_page=100" --paginate > "$MILESTONES_CACHE"
  fi
  echo "$num"
}

# ----- Issue index (build once, lookup locally)
build_issue_index() {
  log "==> Fetching existing issues from $REPO"
  gh api "repos/$REPO/issues?state=all&per_page=100" --paginate \
    --jq '.[] | select(.pull_request == null) | "\(.number)\t\(.title)"' \
    > "$ISSUES_INDEX" || true
  local n
  n=$(wc -l < "$ISSUES_INDEX" | tr -d ' ')
  log "    indexed $n existing issues"
}

find_issue_number_by_title() {
  local title="$1"
  awk -F '\t' -v t="$title" '$2 == t { print $1; exit }' "$ISSUES_INDEX"
}

# ----- Body builder
build_body() {
  local epic="$1" story="$2" title="$3" body_md="$4" line_no="$5"
  local milestone
  milestone=$(milestone_for_epic "$epic")
  cat <<EOF
> Auto-synced from \`$EPICS_FILE\` by \`scripts/sync-epics-to-github.sh\`. Manual edits below this line will be **overwritten** on next sync; status label drift is preserved.

**Epic:** $epic &middot; **Milestone:** $milestone

---

${body_md%$'\n'}

---

**Source:** [\`$EPICS_FILE\` line $line_no](https://github.com/$REPO/blob/$BRANCH_FOR_LINKS/$EPICS_FILE#L$line_no)
EOF
}

# ----- Process a single story
process_story() {
  local epic="$1" story="$2" title="$3" body_md="$4" line_no="$5"
  local milestone="m_$(milestone_for_epic "$epic")"
  local epic_label="epic:$epic"
  local milestone_label="milestone:$(milestone_for_epic "$epic")"
  local type_label="type:story"
  local status_label="status:backlog"

  local issue_title="[Story $epic.$story] $title"
  local body
  body=$(build_body "$epic" "$story" "$title" "$body_md" "$line_no")
  local existing
  existing=$(find_issue_number_by_title "$issue_title")

  if [[ -n "$existing" ]]; then
    log "  ~ #$existing  [Story $epic.$story] (update)"
    # Update body + ensure non-status labels are applied. Do NOT touch status:*.
    if [[ "$DRY_RUN" == "1" ]]; then
      log "[dry-run] gh issue edit $existing --body-file - --add-label $epic_label,$milestone_label,$type_label --milestone $(milestone_for_epic "$epic")"
    else
      printf '%s' "$body" | gh issue edit "$existing" --repo "$REPO" \
        --body-file - \
        --add-label "$epic_label" \
        --add-label "$milestone_label" \
        --add-label "$type_label" \
        --milestone "$(milestone_for_epic "$epic")" >/dev/null
    fi
  else
    log "  + new   [Story $epic.$story] (create)"
    if [[ "$DRY_RUN" == "1" ]]; then
      log "[dry-run] gh issue create --title \"$issue_title\" --milestone $(milestone_for_epic "$epic") --label $epic_label,$milestone_label,$type_label,$status_label"
    else
      printf '%s' "$body" | gh issue create --repo "$REPO" \
        --title "$issue_title" \
        --body-file - \
        --milestone "$(milestone_for_epic "$epic")" \
        --label "$epic_label" \
        --label "$milestone_label" \
        --label "$type_label" \
        --label "$status_label" >/dev/null
    fi
  fi
}

# ===========================================================================

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

LABELS_CACHE="$TMPDIR/labels.txt"
MILESTONES_CACHE="$TMPDIR/milestones.json"
ISSUES_INDEX="$TMPDIR/issues.tsv"

# Prime caches
log "==> Priming labels cache"
gh label list --repo "$REPO" --limit 200 --json name --jq '.[].name' > "$LABELS_CACHE" 2>/dev/null || : > "$LABELS_CACHE"

log "==> Ensuring base label set"
for n in 1 2 3 4 5 6 7 8 9 10 11 12 13; do
  ensure_label "epic:$n" "0e8a16" "Epic $n"
done
ensure_label "milestone:v0.1"  "1d76db" "Milestone v0.1 Alpha"
ensure_label "milestone:v0.5"  "5319e7" "Milestone v0.5 Beta"
ensure_label "milestone:v1.0"  "b60205" "Milestone v1.0"
ensure_label "type:story"      "c5def5" "Story (epic decomposition)"
ensure_label "status:backlog"  "ededed" "Status: backlog (default for synced stories)"

log "==> Priming milestones cache"
gh api "repos/$REPO/milestones?state=all&per_page=100" --paginate > "$MILESTONES_CACHE" 2>/dev/null || echo '[]' > "$MILESTONES_CACHE"

log "==> Ensuring milestones"
ensure_milestone "v0.1" >/dev/null
ensure_milestone "v0.5" >/dev/null
ensure_milestone "v1.0" >/dev/null

build_issue_index

# Parse epics.md and emit one process_story call per Story heading.
log "==> Parsing $EPICS_FILE"

current_epic=""
current_story=""
current_title=""
current_line=""
current_body=""
in_epic_list=0
stories_seen=0

flush() {
  if [[ -n "${current_epic:-}" && -n "${current_story:-}" ]]; then
    process_story "$current_epic" "$current_story" "$current_title" "$current_body" "$current_line"
    stories_seen=$((stories_seen + 1))
  fi
  current_epic=""; current_story=""; current_title=""; current_body=""; current_line=""
}

line_no=0
while IFS= read -r line || [[ -n "$line" ]]; do
  line_no=$((line_no + 1))

  # The first "## Epic List" section contains epic overview *Story* refs are absent there;
  # we only consume Story headings under the per-epic deep sections that come after Epic List.
  if [[ "$line" =~ ^##[[:space:]]Epic[[:space:]]List ]]; then
    in_epic_list=1; flush; continue
  fi
  # Epic List ends at the first "---" or next "## " block. We treat the deep epic sections
  # as starting at "## Epic <N>:" headings.
  if [[ "$line" =~ ^##[[:space:]]Epic[[:space:]]([0-9]+):[[:space:]] ]]; then
    in_epic_list=0; flush; continue
  fi
  if [[ "$line" =~ ^##[[:space:]] ]]; then
    # any other ##, flush
    flush; continue
  fi

  if (( in_epic_list )); then
    continue
  fi

  if [[ "$line" =~ ^###[[:space:]]Story[[:space:]]([0-9]+)\.([0-9]+[a-z]?):[[:space:]](.+)$ ]]; then
    flush
    current_epic="${BASH_REMATCH[1]}"
    current_story="${BASH_REMATCH[2]}"
    current_title="${BASH_REMATCH[3]}"
    current_line="$line_no"
    current_body=""
    continue
  fi

  if [[ -n "$current_story" ]]; then
    current_body+="$line"$'\n'
  fi
done < "$EPICS_FILE"

flush

log "==> Done. Processed $stories_seen stories."

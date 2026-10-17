#!/usr/bin/env bash
# Validate one Smartpowers project without crossing project boundaries.
#
#   validate_workflow.sh [--root <repo>] --list
#   validate_workflow.sh [--root <repo>] --project <id>
#                        [--phase requirements|design|plan|implementation]
#
# Prints `OK: ...` and exits 0, or prints `ERROR: ...` and exits 1. Usage
# errors exit 2.
set -eu

prog=$(basename -- "$0")
projects_path=docs/smartpowers/projects

usage() {
  echo "usage: $prog [-h] [--root ROOT] [--project PROJECT] [--list]"
  echo "       [--phase {requirements,design,plan,implementation}]"
}

usage_error() {
  usage >&2
  echo "$prog: error: $1" >&2
  exit 2
}

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

is_project_id() {
  printf '%s\n' "$1" | LC_ALL=C grep -Eqx '[0-9]{8}-[0-9]{6}-[a-z0-9]+(-[a-z0-9]+)*'
}

# read_required <path>: the file's text with CRLF read as LF.
read_required() {
  [ -f "$1" ] || fail "Missing required file: $1"
  tr -d '\r' <"$1"
}

# field <text> <name>: the first `Name: value` line's trimmed value, matching
# the name case-insensitively. Empty when absent.
field() {
  printf '%s\n' "$1" \
    | LC_ALL=C grep -i -m 1 "^$2:[[:space:]]*[^[:space:]]" \
    | sed -e 's/^[^:]*:[[:space:]]*//' -e 's/[[:space:]]*$//' || true
}

lower() {
  printf '%s' "$1" | LC_ALL=C tr 'A-Z' 'a-z'
}

# ids <text>: sorted unique REQ-### and NFR-### IDs. An ID must not touch an
# ASCII letter, digit, or underscore; any other neighbor (spaces, punctuation,
# Japanese text) ends it.
ids() {
  printf '%s\n' "$1" | LC_ALL=C awk '{
    s = $0
    while (match(s, /(REQ|NFR)-[0-9][0-9][0-9]/)) {
      before = RSTART > 1 ? substr(s, RSTART - 1, 1) : ""
      after = substr(s, RSTART + RLENGTH, 1)
      if (before !~ /[A-Za-z0-9_]/ && after !~ /[A-Za-z0-9_]/) print substr(s, RSTART, RLENGTH)
      s = substr(s, RSTART + 1)
    }
  }' | LC_ALL=C sort -u
}

require_project() { # path text project_id
  local actual
  actual=$(field "$2" Project)
  [ -n "$actual" ] || fail "Missing 'Project:' field in $1"
  [ "$actual" = "$3" ] || fail "$1 belongs to project $actual, expected $3"
}

require_status() { # path text expected
  local actual
  actual=$(field "$2" Status)
  [ -n "$actual" ] || fail "Missing 'Status:' field in $1"
  [ "$(lower "$actual")" = "$(lower "$3")" ] \
    || fail "$1 must have Status: $3; found Status: $actual"
  if [ "$(lower "$3")" = approved ]; then
    printf '%s\n' "$2" | LC_ALL=C grep -iq '^Approved:.' || fail "Missing 'Approved:' record in $1"
  fi
}

require_any_status() { # path text
  [ -n "$(field "$2" Status)" ] || fail "Missing 'Status:' field in $1"
}

require_coverage() { # source expected_ids target text
  local missing
  missing=$(ids "$4" | LC_ALL=C comm -23 <(printf '%s\n' "$2") - | paste -sd ',' - | sed 's/,/, /g')
  [ -z "$missing" ] || fail "$3 does not cover IDs from $1: $missing"
}

# validate_requirements <require_approved>: sets $requirements and $prd_text.
validate_requirements() {
  local path="$project_root/PRD.md"
  prd_text=$(read_required "$path")
  require_project "$path" "$prd_text" "$project_id"
  requirements=$(ids "$prd_text")
  [ -n "$requirements" ] || fail "No REQ-### or NFR-### IDs found in $path"
  if [ "$1" = true ]; then
    require_status "$path" "$prd_text" Approved
  else
    require_any_status "$path" "$prd_text"
  fi
}

# An approved PRD may record `Design: Skipped - <reason>` for small
# non-feature work; then the design doc is not required.
design_skipped() {
  case $(lower "$(field "$prd_text" Design)") in
    skipped*) [ ! -e "$project_root/DESIGN_DOC.md" ] ;;
    *) return 1 ;;
  esac
}

validate_design() { # require_approved
  local path="$project_root/DESIGN_DOC.md" text
  design_skipped && return 0
  text=$(read_required "$path")
  require_project "$path" "$text" "$project_id"
  require_coverage "$project_root/PRD.md" "$requirements" "$path" "$text"
  if [ "$1" = true ]; then
    require_status "$path" "$text" Approved
  else
    require_any_status "$path" "$text"
  fi
}

# task_files: sets $tasks to the numbered task files, one path per line.
task_files() {
  local task_dir="$project_root/tasks" path name invalid="" numbers
  [ -d "$task_dir" ] || fail "Missing required directory: $task_dir"
  tasks=""
  for path in "$task_dir"/*.md; do
    [ -f "$path" ] || continue
    name=$(basename -- "$path")
    [ "$name" = PLAN.md ] && continue
    tasks="$tasks$path"$'\n'
    printf '%s\n' "$name" | LC_ALL=C grep -Eqx '[0-9]{2}_[a-z0-9_]+\.md' || invalid="$invalid, $name"
  done
  [ -z "$invalid" ] || fail "Task filenames must match NN_lowercase_snake_case.md: ${invalid#, }"
  [ -n "$tasks" ] || fail "No numbered task files found in $task_dir"
  numbers=$(printf '%s' "$tasks" | sed 's|.*/\([0-9][0-9]\)_[^/]*$|\1|')
  [ "$(printf '%s\n' "$numbers" | sort | uniq -d)" = "" ] || fail "Task number prefixes must be unique"
}

validate_plan() { # require_approved
  local plan_path="$project_root/tasks/PLAN.md" plan_text path text combined=""
  plan_text=$(read_required "$plan_path")
  require_project "$plan_path" "$plan_text" "$project_id"
  if [ "$1" = true ]; then
    require_status "$plan_path" "$plan_text" Approved
  else
    require_any_status "$plan_path" "$plan_text"
  fi

  task_files
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    text=$(read_required "$path")
    require_project "$path" "$text" "$project_id"
    require_any_status "$path" "$text"
    [ -n "$(ids "$text")" ] || fail "No requirement IDs found in $path"
    combined="$combined$text"$'\n'
  done <<EOF
$tasks
EOF
  require_coverage "$project_root/PRD.md" "$requirements" "$project_root/tasks" "$combined"
}

# Latest modification time (UNIX seconds) of any file under a directory.
latest_mtime() {
  if stat -c %Y "$1" >/dev/null 2>&1; then
    find "$1" -type f -exec stat -c %Y {} + # GNU
  else
    find "$1" -type f -exec stat -f %m {} + # BSD
  fi | sort -n | tail -n 1
}

iso_time() { # seconds
  local offset stamp
  stamp=$(date -r "$1" +%Y-%m-%dT%H:%M:%S 2>/dev/null) || stamp=$(date -d "@$1" +%Y-%m-%dT%H:%M:%S)
  offset=$(date -r "$1" +%z 2>/dev/null) || offset=$(date -d "@$1" +%z)
  printf '%s%s:%s' "$stamp" "${offset%??}" "${offset#???}"
}

list_projects() {
  local projects_root="$root/$projects_path" dir id metadata status title updated found=false
  if [ -d "$projects_root" ]; then
    for dir in "$projects_root"/*/; do
      [ -d "$dir" ] || continue
      id=$(basename -- "$dir")
      is_project_id "$id" || continue
      found=true
      if [ ! -f "$dir/PROJECT.md" ]; then
        printf '%s\tInvalid (missing PROJECT.md)\n' "$id"
        continue
      fi
      metadata=$(tr -d '\r' <"$dir/PROJECT.md")
      status=$(field "$metadata" Status)
      title=$(printf '%s\n' "$metadata" | sed -n '1{s/^# //p;}' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
      case $(printf '%s\n' "$metadata" | head -n 1) in
        "# "*) ;;
        *) title=$id ;;
      esac
      updated=$(iso_time "$(latest_mtime "$dir")")
      printf '%s\t%s\t%s\t%s\n' "$id" "${status:-Unknown}" "$updated" "$title"
    done
  fi
  $found || echo "No Smartpowers projects found"
}

root=$PWD
project_id=""
list=false
phase=implementation
while [ "$#" -gt 0 ]; do
  case $1 in
    -h | --help)
      usage
      exit 0
      ;;
    --list)
      list=true
      shift
      ;;
    --root | --project | --phase)
      [ "$#" -ge 2 ] || usage_error "argument $1: expected one argument"
      case $1 in
        --root) root=$2 ;;
        --project) project_id=$2 ;;
        --phase) phase=$2 ;;
      esac
      shift 2
      ;;
    --root=* | --project=* | --phase=*)
      case $1 in
        --root=*) root=${1#*=} ;;
        --project=*) project_id=${1#*=} ;;
        --phase=*) phase=${1#*=} ;;
      esac
      shift
      ;;
    *) usage_error "unrecognized arguments: $1" ;;
  esac
done
case $phase in
  requirements | design | plan | implementation) ;;
  *) usage_error "argument --phase: invalid choice: '$phase' (choose from 'requirements', 'design', 'plan', 'implementation')" ;;
esac
if [ -d "$root" ]; then
  root=$(CDPATH= cd -- "$root" && pwd -P)
fi

if $list; then
  list_projects
  exit 0
fi
[ -n "$project_id" ] || usage_error "--project is required unless --list is used"

is_project_id "$project_id" || fail "Project ID must match YYYYMMDD-HHMMSS-lowercase-kebab-slug"
project_root="$root/$projects_path/$project_id"
metadata=$(read_required "$project_root/PROJECT.md")
require_project "$project_root/PROJECT.md" "$metadata" "$project_id"

case $phase in
  requirements)
    validate_requirements false
    ;;
  design)
    validate_requirements true
    validate_design false
    ;;
  plan)
    validate_requirements true
    validate_design true
    validate_plan false
    ;;
  implementation)
    validate_requirements true
    validate_design true
    validate_plan true
    ;;
esac

echo "OK: $project_id $phase artifacts are structurally valid"

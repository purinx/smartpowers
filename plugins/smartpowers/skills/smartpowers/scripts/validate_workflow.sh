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

# task_files: sets $tasks to the numbered task files, one path per line, and
# $numbers to their task numbers, space separated.
task_files() {
  local task_dir="$project_root/tasks" path name invalid=""
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
  numbers=$(printf '%s\n' "$numbers" | paste -sd ' ' -)
}

# task_refs <text>: the sorted unique two-digit task numbers in a dependency
# list, space separated, like the board's task_refs: `01`, `Task 01`, and
# links to `NN_name.md` files all count. `None`, `-`, and empty mean none.
task_refs() {
  case $(lower "$1") in
    '' | none | -) return 0 ;;
  esac
  printf '%s\n' "$1" \
    | LC_ALL=C sed -E 's/(^|[^A-Za-z0-9_])([0-9][0-9])_[A-Za-z0-9_]*\.md/\1\2/g' \
    | LC_ALL=C tr -c 'A-Za-z0-9_' '\n' \
    | { LC_ALL=C grep -Ex '[0-9]{2}' || true; } \
    | LC_ALL=C sort -u | paste -sd ' ' -
}

# list_or_none <numbers>: `01, 02`, or `none` when empty.
list_or_none() {
  if [ -n "$1" ]; then
    printf '%s' "$1" | sed 's/ /, /g'
  else
    printf none
  fi
}

has_word() { # space-separated-list word
  case " $1 " in
    *" $2 "*) return 0 ;;
    *) return 1 ;;
  esac
}

# deps_of <number>: the task's dependencies from $graph (`NN:deps` lines).
deps_of() {
  printf '%s' "$graph" | sed -n "s/^$1://p"
}

# check_cycles: fails when the dependencies in $graph form a cycle. Tasks
# whose dependencies are all removed are removed until none can be; then
# tasks that nothing left depends on are trimmed so the cycle is reported.
check_cycles() {
  local left="$numbers" removed="" kept n d all progress=true
  while $progress; do
    progress=false
    kept=""
    for n in $left; do
      all=true
      for d in $(deps_of "$n"); do
        has_word "$removed" "$d" || all=false
      done
      if $all; then
        removed="$removed $n"
        progress=true
      else
        kept="$kept $n"
      fi
    done
    left=$kept
  done
  [ -n "$left" ] || return 0
  progress=true
  while $progress; do
    progress=false
    kept=""
    for n in $left; do
      all=false
      for d in $left; do
        if has_word "$(deps_of "$d")" "$n"; then all=true; fi
      done
      if $all; then kept="$kept $n"; else progress=true; fi
    done
    left=$kept
  done
  fail "Dependency cycle among tasks: $(list_or_none "${left# }")"
}

# plan_rows <plan text>: for the first table whose header row has a
# `Depends on` column, prints `table`, then `NN<TAB>cell` for each row that
# links a task file, where cell is the row's `Depends on` cell. Columns are
# found by header name, so others such as `Parallel group` are ignored.
plan_rows() {
  printf '%s\n' "$1" | LC_ALL=C awk -F '|' '
    function trim(s) { gsub(/^[ \t]+|[ \t]+$/, "", s); return s }
    state == 2 { next }
    /^[ \t]*\|/ {
      # An escaped pipe (`\|`) is cell text, not a column delimiter.
      gsub(/\\\|/, "\001")
      if (state == 0) {
        for (i = 1; i <= NF; i++) if (tolower(trim($i)) == "depends on") col = i
        if (col) { state = 1; print "table" }
        next
      }
      if (match($0, /(^|[^A-Za-z0-9_])[0-9][0-9]_[a-z0-9_]+\.md/)) {
        link = substr($0, RSTART, RLENGTH)
        sub(/^[^0-9]/, "", link)
        print substr(link, 1, 2) "\t" $col
      }
      next
    }
    state == 1 { state = 2 }
  '
}

# check_plan_table <plan path> <plan text>: the execution order table must
# list every task, with the same dependencies as its task file.
check_plan_table() {
  local rows number cell listed="" expected actual
  rows=$(plan_rows "$2")
  [ "$(printf '%s\n' "$rows" | head -n 1)" = table ] \
    || fail "No execution order table with a Depends on column in $1"
  while IFS=$'\t' read -r number cell; do
    [ "$number" != table ] || continue
    has_word "$numbers" "$number" \
      || fail "Execution order table in $1 lists task $number, which has no task file"
    expected=$(deps_of "$number")
    actual=$(task_refs "$(printf '%s' "$cell" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')")
    [ "$expected" = "$actual" ] \
      || fail "Task $number depends on $(list_or_none "$expected") in its task file but on $(list_or_none "$actual") in the execution order table in $1"
    listed="$listed $number"
  done <<EOF
$rows
EOF
  for number in $numbers; do
    has_word "$listed" "$number" || fail "Task $number is missing from the execution order table in $1"
  done
}

validate_plan() { # require_approved
  local plan_path="$project_root/tasks/PLAN.md" plan_text path text combined="" number dep deps
  graph=""
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
    number=$(basename -- "$path" | cut -c 1-2)
    deps=$(task_refs "$(field "$text" 'Depends on')")
    for dep in $deps; do
      has_word "$numbers" "$dep" || fail "Task $number depends on missing task $dep in $path"
    done
    graph="$graph$number:$deps"$'\n'
  done <<EOF
$tasks
EOF
  check_cycles
  check_plan_table "$plan_path" "$plan_text"
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

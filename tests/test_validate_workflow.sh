# scripts/validate_workflow.sh checks one project's artifacts per phase.

ID=20260908-183045-auth-refresh

project() {
  printf '%s' "$TEST_TMP/docs/smartpowers/projects/$ID"
}

doc() { # relative-path text
  write "$(project)/$1" "$2"
}

validate() { # phase
  # $BASH: the shell running the suite, so `/bin/bash tests/run.sh` checks 3.2.
  run "$BASH" "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --project "$ID" --phase "$1"
}

# plan <table rows>: an approved PLAN.md whose execution order table has the
# given rows after its header.
plan() {
  doc tasks/PLAN.md "# Plan\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\n## Execution order\n\n| Order | Task | Depends on | Requirements |\n| --- | --- | --- | --- |\n$1\n## Notes\n\n| Task | Depends on |\n| --- | --- |\n| [x](07_unrelated.md) | 08 |\n"
}

task() { # number name depends-on
  doc "tasks/$1_$2.md" "# $2\n\nStatus: Planned\nProject: $ID\nRequirements: REQ-001, NFR-001\nDepends on: $3\n"
}

valid_project() {
  doc PROJECT.md "# Auth refresh\n\nProject: $ID\nStatus: Active\n"
  doc PRD.md "# PRD\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\nREQ-001: login\nNFR-001: latency\n"
  doc DESIGN_DOC.md "# Design\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\nREQ-001 via auth flow\nNFR-001 via cache\n"
  plan "| 01 | [Auth](01_auth.md) | None | REQ-001 |\n"
  doc tasks/01_auth.md "# Auth\n\nStatus: Planned\nProject: $ID\nRequirements: REQ-001, NFR-001\n"
}

# diamond_project: 01 -> 02, 03 -> 04, written in each accepted form.
diamond_project() {
  valid_project
  task 02 left "01"
  task 03 right "Task 01"
  task 04 join "[left](02_left.md), [Task 03](03_right.md)"
  plan "| 01 | [Auth](01_auth.md) | - | REQ-001 |\n| 02 | [Left](02_left.md) | 01 | REQ-001 |\n| 03 | [Right](03_right.md) | Task 01 | REQ-001 |\n| 04 | [Join](04_join.md) | 03, 02 | NFR-001 |\n"
}

test_valid_diamond_plan() {
  diamond_project
  validate plan
  assert_status 0
  validate implementation
  assert_status 0
}

test_rejects_dependency_on_missing_task() {
  diamond_project
  task 03 right "01, 09"
  validate plan
  assert_status 1
  assert_eq "ERROR: Task 03 depends on missing task 09 in $(cd "$TEST_TMP" && pwd -P)/docs/smartpowers/projects/$ID/tasks/03_right.md" "$errors"
}

test_rejects_dependency_cycle() {
  diamond_project
  task 02 left "01, 05"
  task 05 loop "04"
  plan "| 01 | [Auth](01_auth.md) | None | REQ-001 |\n| 02 | [Left](02_left.md) | 01, 05 | REQ-001 |\n| 03 | [Right](03_right.md) | 01 | REQ-001 |\n| 04 | [Join](04_join.md) | 02, 03 | NFR-001 |\n| 05 | [Loop](05_loop.md) | 04 | NFR-001 |\n"
  validate plan
  assert_status 1
  assert_eq "ERROR: Dependency cycle among tasks: 02, 04, 05" "$errors"
}

test_rejects_table_that_disagrees_with_task_file() {
  diamond_project
  plan "| 01 | [Auth](01_auth.md) | None | REQ-001 |\n| 02 | [Left](02_left.md) | 01 | REQ-001 |\n| 03 | [Right](03_right.md) | 01 | REQ-001 |\n| 04 | [Join](04_join.md) | 02 | NFR-001 |\n"
  validate plan
  assert_status 1
  assert_contains "$errors" "Task 04 depends on 02, 03 in its task file but on 02 in the execution order table in"
}

test_rejects_task_missing_from_table() {
  diamond_project
  plan "| 01 | [Auth](01_auth.md) | None | REQ-001 |\n| 02 | [Left](02_left.md) | 01 | REQ-001 |\n| 04 | [Join](04_join.md) | 02, 03 | NFR-001 |\n"
  validate plan
  assert_status 1
  assert_contains "$errors" "Task 03 is missing from the execution order table in"
}

test_rejects_plan_without_depends_on_table() {
  valid_project
  doc tasks/PLAN.md "# Plan\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\n| Order | Task |\n| --- | --- |\n| 01 | [Auth](01_auth.md) |\n"
  validate plan
  assert_status 1
  assert_eq "ERROR: No execution order table with a Depends on column in $(cd "$TEST_TMP" && pwd -P)/docs/smartpowers/projects/$ID/tasks/PLAN.md" "$errors"
}

test_ignores_parallel_group_column_and_field() {
  diamond_project
  doc tasks/03_right.md "# right\n\nStatus: Planned\nProject: $ID\nRequirements: REQ-001\nParallel group: A\nDepends on: 01\n"
  doc tasks/PLAN.md "# Plan\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\n| Order | Task | Parallel group | Depends on | Requirements |\n| --- | --- | --- | --- | --- |\n| 01 | [Auth](01_auth.md) | A | None | REQ-001 |\n| 02 | [Left](02_left.md) | B | 01 | REQ-001 |\n| 03 | [Right](03_right.md) | B | 01 | REQ-001 |\n| 04 | [Join](04_join.md) | C | 02, 03 | NFR-001 |\n"
  validate implementation
  assert_status 0
}

test_escaped_pipe_in_task_title_is_not_a_column() {
  diamond_project
  plan "| 01 | [Auth \\\\| refresh](01_auth.md) | None | REQ-001 |\n| 02 | [Left](02_left.md) | 01 | REQ-001 |\n| 03 | [Right \\\\| side](03_right.md) | 01 | REQ-001 |\n| 04 | [Join](04_join.md) | 02, 03 | NFR-001 |\n"
  grep -qF 'Auth \| refresh' "$(project)/tasks/PLAN.md" || fail "fixture lost its escaped pipe"
  validate plan
  assert_status 0
}

test_valid_plan() {
  valid_project
  validate implementation
  assert_status 0
  assert_eq "OK: $ID implementation artifacts are structurally valid" "$output"
}

test_design_reports_missing_requirement() {
  valid_project
  doc DESIGN_DOC.md "# Design\n\nStatus: Draft\nProject: $ID\n\nREQ-001 via auth flow\n"
  validate design
  assert_status 1
  assert_eq "ERROR: $(cd "$TEST_TMP" && pwd -P)/docs/smartpowers/projects/$ID/DESIGN_DOC.md does not cover IDs from $(cd "$TEST_TMP" && pwd -P)/docs/smartpowers/projects/$ID/PRD.md: NFR-001" "$errors"
}

test_plan_rejects_invalid_task_filename() {
  valid_project
  rm "$(project)/tasks/01_auth.md"
  doc tasks/auth-task.md "# Auth\n\nStatus: Planned\nProject: $ID\nRequirements: REQ-001, NFR-001\n"
  validate implementation
  assert_status 1
  assert_contains "$errors" "Task filenames must match NN_lowercase_snake_case.md: auth-task.md"
}

test_approved_status_requires_approval_record() {
  valid_project
  doc tasks/PLAN.md "# Plan\n\nStatus: Approved\nProject: $ID\n"
  validate implementation
  assert_status 1
  assert_contains "$errors" "Missing 'Approved:' record"
}

test_draft_plan_can_be_validated_before_review() {
  valid_project
  doc tasks/PLAN.md "# Plan\n\nStatus: Draft\nProject: $ID\n\n| Task | Depends on |\n| --- | --- |\n| [Auth](01_auth.md) | None |\n"
  validate plan
  assert_status 0
  validate implementation
  assert_status 1
  assert_contains "$errors" "must have Status: Approved; found Status: Draft"
}

test_rejects_artifact_from_another_project() {
  valid_project
  doc PRD.md "# PRD\n\nStatus: Draft\nProject: 20260908-190000-other-work\nREQ-001: other\n"
  validate requirements
  assert_status 1
  assert_contains "$errors" "belongs to project 20260908-190000-other-work, expected $ID"
}

test_selected_project_is_isolated_from_other_projects() {
  valid_project
  write "$TEST_TMP/docs/smartpowers/projects/20260908-190000-other-work/PROJECT.md" "broken"
  validate implementation
  assert_status 0
}

test_reads_crlf_documents_like_lf() {
  valid_project
  local file
  for file in PROJECT.md PRD.md DESIGN_DOC.md tasks/PLAN.md tasks/01_auth.md; do
    awk '{ printf "%s\r\n", $0 }' "$(project)/$file" >"$TEST_TMP/crlf"
    mv "$TEST_TMP/crlf" "$(project)/$file"
  done
  # Count bytes: Git Bash's sed and grep may hide carriage returns.
  [ "$(tr -d '\r' <"$(project)/PRD.md" | wc -c)" -lt "$(wc -c <"$(project)/PRD.md")" ] \
    || fail "the fixture is not CRLF"
  validate implementation
  assert_status 0
}

test_ids_end_at_non_word_neighbors() {
  valid_project
  doc PRD.md "# PRD\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\n要件REQ-001の説明、(NFR-001)\nxREQ-009 REQ-0099 REQ-008_\n"
  validate implementation
  assert_status 0
  # REQ-001 counts even though Japanese text touches it.
  doc tasks/01_auth.md "# Auth\n\nStatus: Planned\nProject: $ID\nRequirements: NFR-001\n"
  validate implementation
  assert_status 1
  assert_contains "$errors" ": REQ-001"
}

test_skipped_design_needs_no_design_doc() {
  valid_project
  rm "$(project)/DESIGN_DOC.md"
  validate plan
  assert_status 1
  assert_contains "$errors" "Missing required file"
  doc PRD.md "# PRD\n\nStatus: Approved\nDesign: Skipped - helper script\nProject: $ID\nApproved: 2026-09-08 by user\n\nREQ-001: login\nNFR-001: latency\n"
  validate implementation
  assert_status 0
  # Tasks must still cover the PRD.
  doc tasks/01_auth.md "# Auth\n\nStatus: Planned\nProject: $ID\nRequirements: REQ-001\n"
  validate implementation
  assert_status 1
  assert_contains "$errors" "does not cover IDs from"
}

test_lists_projects() {
  valid_project
  mkdir -p "$TEST_TMP/docs/smartpowers/projects/20260909-100000-no-metadata"
  mkdir -p "$TEST_TMP/docs/smartpowers/projects/not-a-project"
  run "$BASH" "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --list
  assert_status 0
  local first second
  first=$(printf '%s\n' "$output" | sed -n 1p)
  second=$(printf '%s\n' "$output" | sed -n 2p)
  printf '%s\n' "$first" | grep -Eqx "$ID	Active	[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}[+-][0-9]{2}:[0-9]{2}	Auth refresh" \
    || fail "unexpected row: $first"
  assert_eq "20260909-100000-no-metadata	Invalid (missing PROJECT.md)" "$second"
  assert_eq 2 "$(printf '%s\n' "$output" | wc -l | tr -d ' ')"
}

test_usage_errors_exit_2() {
  run "$BASH" "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP"
  assert_status 2
  assert_contains "$errors" "--project is required unless --list is used"
  run "$BASH" "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --project "$ID" --phase review
  assert_status 2
  assert_contains "$errors" "invalid choice: 'review'"
  run "$BASH" "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --project Bad_ID
  assert_status 1
  assert_contains "$errors" "Project ID must match"
}

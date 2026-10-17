# scripts/validate_workflow.sh checks one project's artifacts per phase.

ID=20260908-183045-auth-refresh

project() {
  printf '%s' "$TEST_TMP/docs/smartpowers/projects/$ID"
}

doc() { # relative-path text
  write "$(project)/$1" "$2"
}

validate() { # phase
  run bash "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --project "$ID" --phase "$1"
}

valid_project() {
  doc PROJECT.md "# Auth refresh\n\nProject: $ID\nStatus: Active\n"
  doc PRD.md "# PRD\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\nREQ-001: login\nNFR-001: latency\n"
  doc DESIGN_DOC.md "# Design\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n\nREQ-001 via auth flow\nNFR-001 via cache\n"
  doc tasks/PLAN.md "# Plan\n\nStatus: Approved\nProject: $ID\nApproved: 2026-09-08 by user\n"
  doc tasks/01_auth.md "# Auth\n\nStatus: Planned\nProject: $ID\nRequirements: REQ-001, NFR-001\n"
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
  doc tasks/PLAN.md "# Plan\n\nStatus: Draft\nProject: $ID\n"
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
    sed 's/$/\r/' "$(project)/$file" >"$TEST_TMP/crlf" && mv "$TEST_TMP/crlf" "$(project)/$file"
  done
  grep -q $'\r' "$(project)/PRD.md" || fail "the fixture is not CRLF"
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
  run bash "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --list
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
  run bash "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP"
  assert_status 2
  assert_contains "$errors" "--project is required unless --list is used"
  run bash "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --project "$ID" --phase review
  assert_status 2
  assert_contains "$errors" "invalid choice: 'review'"
  run bash "$SCRIPTS/validate_workflow.sh" --root "$TEST_TMP" --project Bad_ID
  assert_status 1
  assert_contains "$errors" "Project ID must match"
}

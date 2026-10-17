# scripts/init_project.sh creates timestamped project workspaces.

init_project() {
  run env TZ=JST-9 SMARTPOWERS_NOW="${NOW:-1788859845}" \
    bash "$SCRIPTS/init_project.sh" --root "$TEST_TMP" "$@"
}

projects() {
  ls "$TEST_TMP/docs/smartpowers/projects"
}

test_creates_timestamped_project_metadata() {
  # 1788859845 is 2026-09-08T18:30:45+09:00.
  init_project --title "OAuth Refresh"
  assert_status 0
  local id=20260908-183045-oauth-refresh
  local path="$TEST_TMP/docs/smartpowers/projects/$id"
  assert_eq "Project: $id
Path: $(cd "$TEST_TMP" && pwd -P)/docs/smartpowers/projects/$id" "$output"
  assert_eq "# OAuth Refresh

Project: $id
Status: Active
Created: 2026-09-08T18:30:45+09:00
Slug: oauth-refresh" "$(cat "$path/PROJECT.md")"
}

test_same_slug_at_different_times_creates_distinct_projects() {
  NOW=1788859845 init_project --title Auth --slug auth
  assert_status 0
  NOW=1788859905 init_project --title Auth --slug auth
  assert_status 0
  assert_eq "20260908-183045-auth
20260908-183145-auth" "$(projects)"
}

test_never_overwrites_an_existing_project() {
  init_project --title Auth --slug auth
  echo kept >>"$TEST_TMP/docs/smartpowers/projects/20260908-183045-auth/PROJECT.md"
  init_project --title Other --slug auth
  assert_status 2
  assert_contains "$errors" "Project already exists"
  assert_contains "$(cat "$TEST_TMP/docs/smartpowers/projects/20260908-183045-auth/PROJECT.md")" kept
}

test_non_ascii_title_requires_explicit_slug() {
  init_project --title "認証更新"
  assert_status 2
  assert_contains "$errors" "--slug"
  init_project --title "認証更新" --slug auth-refresh
  assert_status 0
  assert_contains "$(cat "$TEST_TMP/docs/smartpowers/projects/20260908-183045-auth-refresh/PROJECT.md")" "# 認証更新"
}

test_slugifies_punctuation_and_case() {
  init_project --title "  Fix: the API -- v2!  "
  assert_status 0
  NOW=1788859905 init_project --title "Don't stop"
  assert_status 0
  assert_eq "20260908-183045-fix-the-api-v2
20260908-183145-don-t-stop" "$(projects)"
}

test_rejects_invalid_slugs() {
  local slug
  for slug in Auth "auth--refresh" "-auth" "auth_refresh" "$(printf 'a%.0s' $(seq 65))"; do
    init_project --title Auth --slug "$slug"
    assert_status 2
    assert_contains "$errors" "Slug must be 1-64 characters"
  done
  [ ! -d "$TEST_TMP/docs/smartpowers/projects" ] || assert_eq "" "$(projects)"
}

test_rejects_empty_or_multiline_titles() {
  init_project --title "   "
  assert_status 2
  assert_contains "$errors" "Title must be a non-empty single line"
  init_project --title "$(printf 'one\ntwo')"
  assert_status 2
  assert_contains "$errors" "Title must be a non-empty single line"
  run bash "$SCRIPTS/init_project.sh" --root "$TEST_TMP"
  assert_status 2
  assert_contains "$errors" "--title"
}

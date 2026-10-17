# The Codex and Claude Code catalogs and manifests describe the same plugin.
#
# The JSON files are pretty-printed with two-space indents, so a key's indent
# says how deep it is. Their syntax is checked by the board crate's
# tests/manifests.rs, which parses them.

# json_value <file> <indent> <key>: the first string value of <key> at that
# indent (in spaces).
json_value() {
  local pad
  pad=$(printf '%*s' "$2" '')
  sed -n "s/^$pad\"$3\": \"\\([^\"]*\\)\".*/\\1/p" "$REPO_ROOT/$1" | head -n 1
}

test_catalogs_reference_the_same_plugin() {
  local codex=.agents/plugins/marketplace.json claude=.claude-plugin/marketplace.json
  assert_eq smartpowers "$(json_value $codex 2 name)"
  assert_eq smartpowers "$(json_value $claude 2 name)"
  assert_eq smartpowers "$(json_value $codex 6 name)"
  assert_eq smartpowers "$(json_value $claude 6 name)"
  assert_eq ./plugins/smartpowers "$(json_value $codex 8 path)"
  assert_eq ./plugins/smartpowers "$(json_value $claude 6 source)"
}

test_plugin_manifests_stay_in_sync() {
  local codex=plugins/smartpowers/.codex-plugin/plugin.json claude=plugins/smartpowers/.claude-plugin/plugin.json
  local key
  for key in name version description; do
    [ -n "$(json_value $claude 2 $key)" ] || fail "no $key in $claude"
    assert_eq "$(json_value $claude 2 $key)" "$(json_value $codex 2 $key)" "$key differs"
  done
  assert_dir "$REPO_ROOT/plugins/smartpowers/skills"
}

test_board_crate_version_matches_plugin() {
  local version board="$REPO_ROOT/plugins/smartpowers/skills/smartpowers/board"
  version=$(json_value plugins/smartpowers/.claude-plugin/plugin.json 2 version)
  grep -qx "version = \"$version\"" "$board/Cargo.toml" || fail "Cargo.toml is not at $version"
  grep -A1 -x 'name = "smartpowers-board"' "$board/Cargo.lock" | grep -qx "version = \"$version\"" \
    || fail "Cargo.lock is not at $version"
}

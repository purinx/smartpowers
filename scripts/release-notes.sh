#!/usr/bin/env bash
# Check a release tag against the repository and print its release notes.
#
#   scripts/release-notes.sh --tag v0.7.0 [--output notes.md]
#   scripts/release-notes.sh --current      # the version the repository declares
#
# Fails unless every declared version equals the tag and CHANGELOG.md has a
# `## [X.Y.Z]` section. That section's body is the release notes.
# Used by .github/workflows/release.yml (on tag push) and ci.yml.
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
claude_manifest=plugins/smartpowers/.claude-plugin/plugin.json
codex_manifest=plugins/smartpowers/.codex-plugin/plugin.json
cargo_manifest=plugins/smartpowers/skills/smartpowers/board/Cargo.toml

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

usage() {
  sed -n '2,9s/^# \{0,1\}//p' "$0"
}

json_version() {
  sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$root/$1" | head -n 1
}

cargo_version() {
  sed -n 's/^version = "\([^"]*\)".*/\1/p' "$root/$cargo_manifest" | head -n 1
}

tag=""
output=""
while [ $# -gt 0 ]; do
  case $1 in
    --tag) [ $# -ge 2 ] || fail "--tag requires a value"; tag=$2; shift 2 ;;
    --tag=*) tag=${1#--tag=}; shift ;;
    --current) tag="v$(json_version "$claude_manifest")"; shift ;;
    --output) [ $# -ge 2 ] || fail "--output requires a value"; output=$2; shift 2 ;;
    --output=*) output=${1#--output=}; shift ;;
    -h | --help) usage; exit 0 ;;
    *) fail "Unknown argument: $1 (see --help)" ;;
  esac
done
[ -n "$tag" ] || fail "Pass --tag vX.Y.Z or --current (see --help)"

[[ $tag =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]] || fail "Tag '$tag' must look like v1.2.3"
version=${BASH_REMATCH[1]}

mismatches=""
check() {
  if [ "$2" != "$version" ]; then
    mismatches="$mismatches; $1 has ${2:-no version}"
  fi
}
check "$claude_manifest" "$(json_version "$claude_manifest")"
check "$codex_manifest" "$(json_version "$codex_manifest")"
check "$cargo_manifest" "$(cargo_version)"
[ -z "$mismatches" ] || fail "Tag $tag does not match the declared version${mismatches/;/:}"

# The lines after `## [version]` up to the next `## [`, without leading or
# trailing blank lines.
notes=$(awk -v heading="## [$version]" '
  index($0, "## [") == 1 {
    if (inside) exit
    if (index($0, heading) == 1) { inside = 1; found = 1; next }
  }
  inside {
    lines[++count] = $0
    if ($0 ~ /[^[:space:]]/) { if (!first) first = count; last = count }
  }
  END {
    if (!found) exit 2
    for (i = first; first && i <= last; i++) print lines[i]
  }
' "$root/CHANGELOG.md") || fail "CHANGELOG.md has no '## [$version]' section"
[ -n "$notes" ] || fail "CHANGELOG.md section for $version is empty"

if [ -n "$output" ]; then
  printf '%s\n' "$notes" >"$output"
else
  printf '%s\n' "$notes"
fi

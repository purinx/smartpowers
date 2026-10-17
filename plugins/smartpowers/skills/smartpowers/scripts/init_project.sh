#!/usr/bin/env bash
# Create an isolated timestamped Smartpowers project workspace:
# docs/smartpowers/projects/<YYYYMMDD-HHMMSS>-<slug>/PROJECT.md
#
#   init_project.sh [--root <repo>] --title <title> [--slug <slug>]
#
# The slug defaults to an ASCII version of the title. SMARTPOWERS_NOW (UNIX
# seconds) fixes the creation time; it exists for tests.
set -eu

prog=$(basename -- "$0")
projects_path=docs/smartpowers/projects

usage() {
  echo "usage: $prog [-h] [--root ROOT] --title TITLE [--slug SLUG]"
}

error() {
  usage >&2
  echo "$prog: error: $1" >&2
  exit 2
}

# The title as lowercase ASCII words joined by single hyphens. Accented
# letters are transliterated where iconv can; other characters are dropped.
slugify() {
  # ASCII titles skip iconv, whose transliteration marks (Caf'e) would
  # otherwise be indistinguishable from real apostrophes. iconv may be
  # missing, and macOS iconv exits 1 even after transliterating, so judge it
  # by its output; -c drops what it cannot transliterate.
  local ascii=$1
  if printf '%s' "$1" | LC_ALL=C grep -q '[^ -~]'; then
    ascii=$(printf '%s' "$1" | iconv -c -f UTF-8 -t ASCII//TRANSLIT 2>/dev/null) || true
    [ -n "$ascii" ] || ascii=$1
    ascii=$(printf '%s' "$ascii" | LC_ALL=C tr -d "'\"^\`~?")
  fi
  printf '%s' "$ascii" \
    | LC_ALL=C tr -cd '\40-\176' \
    | LC_ALL=C tr 'A-Z' 'a-z' \
    | LC_ALL=C sed -e 's/[^a-z0-9][^a-z0-9]*/-/g' -e 's/^-*//' -e 's/-*$//'
}

valid_slug() {
  [ "${#1}" -le 64 ] && printf '%s\n' "$1" | LC_ALL=C grep -Eqx '[a-z0-9]+(-[a-z0-9]+)*'
}

# date(1) for a UNIX time: BSD takes -r, GNU takes -d @.
format_time() { # seconds format
  date -r "$1" "$2" 2>/dev/null || date -d "@$1" "$2"
}

root=$PWD
title=""
has_title=false
slug=""
while [ "$#" -gt 0 ]; do
  case $1 in
    -h | --help)
      usage
      exit 0
      ;;
    --root | --title | --slug)
      [ "$#" -ge 2 ] || error "argument $1: expected one argument"
      case $1 in
        --root) root=$2 ;;
        --title) title=$2 has_title=true ;;
        --slug) slug=$2 ;;
      esac
      shift 2
      ;;
    --root=* | --title=* | --slug=*)
      value=${1#*=}
      case $1 in
        --root=*) root=$value ;;
        --title=*) title=$value has_title=true ;;
        --slug=*) slug=$value ;;
      esac
      shift
      ;;
    *) error "unrecognized arguments: $1" ;;
  esac
done
$has_title || error "the following arguments are required: --title"

# Trim surrounding whitespace, then require one non-empty line.
title=$(printf '%s' "$title" | sed -e '1s/^[[:space:]]*//' -e '$s/[[:space:]]*$//')
case $title in
  "" | *$'\n'* | *$'\r'*) error "Title must be a non-empty single line" ;;
esac

if [ -n "$slug" ]; then
  slug=$(printf '%s' "$slug" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
else
  slug=$(slugify "$title")
  [ -n "$slug" ] || error "Could not derive an ASCII slug from the title; pass --slug explicitly"
fi
valid_slug "$slug" \
  || error "Slug must be 1-64 characters of lowercase ASCII letters, digits, and single hyphens"

now=${SMARTPOWERS_NOW:-$(date +%s)}
stamp=$(format_time "$now" +%Y%m%d-%H%M%S)
offset=$(format_time "$now" +%z)
created="$(format_time "$now" +%Y-%m-%dT%H:%M:%S)${offset%??}:${offset#???}"

[ -d "$root" ] || error "Root is not a directory: $root"
root=$(CDPATH= cd -- "$root" && pwd -P)
project_id="$stamp-$slug"
project_root="$root/$projects_path/$project_id"
mkdir -p "$root/$projects_path"
# mkdir without -p fails when the project exists, so nothing is overwritten.
mkdir "$project_root" 2>/dev/null || error "Project already exists: $project_root"

printf '# %s\n\nProject: %s\nStatus: Active\nCreated: %s\nSlug: %s\n' \
  "$title" "$project_id" "$created" "$slug" >"$project_root/PROJECT.md"

echo "Project: $project_id"
echo "Path: $project_root"

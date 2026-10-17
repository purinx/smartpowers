# Assertions and helpers for the shell test suite. Sourced by tests/run.sh.
#
# Each test_* function runs in its own subshell with a fresh $TEST_TMP, so an
# assertion fails the current test by exiting that subshell.

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
SCRIPTS="$REPO_ROOT/plugins/smartpowers/skills/smartpowers/scripts"

fail() {
  printf '    %s\n' "$@" >&2
  exit 1
}

assert_eq() { # expected actual [message]
  [ "$1" = "$2" ] || fail "${3:-values differ}" "expected: $1" "actual:   $2"
}

assert_contains() { # haystack needle [message]
  case $1 in
    *"$2"*) ;;
    *) fail "${3:-missing text}" "expected to contain: $2" "actual: $1" ;;
  esac
}

assert_not_contains() { # haystack needle [message]
  case $1 in
    *"$2"*) fail "${3:-unexpected text}" "expected not to contain: $2" "actual: $1" ;;
  esac
}

assert_file() { # path
  [ -f "$1" ] || fail "missing file: $1"
}

assert_dir() { # path
  [ -d "$1" ] || fail "missing directory: $1"
}

# run <command...>: runs the command, capturing $status, $output (stdout) and
# $errors (stderr) without failing the test.
run() {
  local out err
  out="$TEST_TMP/.run-out"
  err="$TEST_TMP/.run-err"
  status=0
  "$@" >"$out" 2>"$err" || status=$?
  output=$(cat "$out")
  errors=$(cat "$err")
}

assert_status() { # expected
  [ "$status" = "$1" ] || fail "expected exit status $1, got $status" "stdout: $output" "stderr: $errors"
}

# write <path> <text>: writes text (printf %b escapes) creating parent dirs.
write() {
  mkdir -p "$(dirname -- "$1")"
  printf '%b' "$2" >"$1"
}

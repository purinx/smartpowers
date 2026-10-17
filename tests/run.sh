#!/usr/bin/env bash
# Run the shell test suite: every test_* function in tests/test_*.sh.
#
#   bash tests/run.sh                         # all test files
#   bash tests/run.sh tests/test_marketplace.sh
#
# Exits non-zero when any test fails, naming each failure and its reason.
set -u

tests_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "$#" -eq 0 ]; then
  set -- "$tests_dir"/test_*.sh
fi

passed=0
failed=0
failures=""
for file in "$@"; do
  name=$(basename -- "$file" .sh)
  # List the file's tests in a throwaway shell so files can't see each other.
  tests=$(bash -c '. "$1"; declare -F' _ "$file" | awk '$3 ~ /^test_/ {print $3}')
  for test in $tests; do
    tmp=$(mktemp -d "${TMPDIR:-/tmp}/smartpowers-test.XXXXXX")
    # Not an `if` condition: that would disable `set -e` inside the subshell.
    (
      export TEST_TMP="$tmp"
      . "$tests_dir/lib.sh"
      . "$file"
      set -e
      "$test"
    )
    if [ "$?" -eq 0 ]; then
      passed=$((passed + 1))
      echo "ok   $name::$test"
    else
      failed=$((failed + 1))
      failures="$failures $name::$test"
      echo "FAIL $name::$test"
    fi
    rm -rf "$tmp"
  done
done

echo
echo "$passed passed, $failed failed"
for failure in $failures; do
  echo "  failed: $failure"
done
[ "$failed" -eq 0 ]

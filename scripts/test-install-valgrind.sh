#!/usr/bin/env bash
# Exercise transient downloads, exhausted retries and the overall timeout offline.
set -euo pipefail
installer=$(cd "$(dirname "$0")" && pwd)/install-valgrind.sh
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir "$scratch/bin"
ln -s /bin/bash "$scratch/bin/bash"
ln -s /usr/bin/env "$scratch/bin/env"
export INSTALL_TEST_BIN="$scratch/bin"
export INSTALL_TEST_STATE="$scratch"
cat > "$scratch/bin/timeout" <<'SH'
#!/bin/bash
if [[ ${INSTALL_TEST_TIMEOUT:-0} == 1 ]]; then exit 124; fi
while [[ $1 == --* ]]; do shift; done
shift
exec "$@"
SH
cat > "$scratch/bin/sudo" <<'SH'
#!/bin/bash
exec "$@"
SH
cat > "$scratch/bin/apt-get" <<'SH'
#!/bin/bash
set -eu
operation=update
for argument in "$@"; do
    if [[ $argument == install ]]; then operation=install; fi
done
count=0
if [[ -f $INSTALL_TEST_STATE/$operation ]]; then
    read -r count < "$INSTALL_TEST_STATE/$operation"
fi
count=$((count + 1))
echo "$count" > "$INSTALL_TEST_STATE/$operation"
if [[ $operation == "${INSTALL_TEST_FAIL_OPERATION:-install}" && $count -le ${INSTALL_TEST_FAILURES:-1} ]]; then
    exit 100
fi
if [[ $operation == install ]]; then
    printf '#!/bin/bash\necho valgrind-test\n' > "$INSTALL_TEST_BIN/valgrind"
    /bin/chmod +x "$INSTALL_TEST_BIN/valgrind"
fi
SH
chmod +x "$scratch/bin/timeout" "$scratch/bin/sudo" "$scratch/bin/apt-get"

PATH="$scratch/bin" /bin/bash "$installer" > "$scratch/recovery.log" 2>&1
[[ $(cat "$scratch/install") == 2 ]]
grep -q 'attempt 1 failed (100)' "$scratch/recovery.log"
# An installed tool must be reused without another apt invocation.
PATH="$scratch/bin" /bin/bash "$installer" > /dev/null
[[ $(cat "$scratch/install") == 2 ]]

for operation in update install; do
    rm -f "$scratch/bin/valgrind" "$scratch/update" "$scratch/install"
    status=0
    INSTALL_TEST_FAIL_OPERATION="$operation" INSTALL_TEST_FAILURES=9 PATH="$scratch/bin" \
        /bin/bash "$installer" > "$scratch/exhausted.log" 2>&1 || status=$?
    [[ $status == 100 && $(cat "$scratch/$operation") == 3 ]]
    [[ ! -e $scratch/bin/valgrind ]]
done

status=0
INSTALL_TEST_TIMEOUT=1 PATH="$scratch/bin" /bin/bash "$installer" > /dev/null 2>&1 || status=$?
[[ $status == 124 ]]
echo "Valgrind setup recovery, reuse, failure and timeout checks passed."

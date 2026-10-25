#!/usr/bin/env bash
# Bound the complete setup, including retries and child processes, to 15 minutes.
set -euo pipefail

if command -v valgrind >/dev/null 2>&1; then
    valgrind --version
    exit 0
fi

if [[ ${1:-} != --bounded ]]; then
    exec timeout --signal=TERM --kill-after=15s 900 bash "$0" --bounded
fi

retry_apt() {
    local operation=$1
    shift
    local attempt status
    for attempt in 1 2 3; do
        echo "Valgrind setup: ${operation}, attempt ${attempt}/3"
        if sudo timeout --signal=TERM --kill-after=15s 240 \
            env DEBIAN_FRONTEND=noninteractive apt-get \
            -o Acquire::Retries=3 \
            -o Acquire::http::Timeout=30 \
            -o Acquire::https::Timeout=30 "$@"; then
            return 0
        else
            status=$?
            echo "Valgrind setup: ${operation} attempt ${attempt} failed (${status})" >&2
        fi
    done
    return "$status"
}

retry_apt update update
retry_apt install install --yes valgrind
valgrind --version

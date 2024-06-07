#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
set -e

# If getpid not in /benchmark, copy it from /regression/getpid/getpid;
# If not there, exit
if [ ! -f /benchmark/getpid ]; then
    if [ -f /regression/getpid/getpid ]; then
        cp /regression/getpid/getpid /benchmark
    else
        echo "Error: getpid binary not found"
        exit 1
    fi
fi

/benchmark/getpid

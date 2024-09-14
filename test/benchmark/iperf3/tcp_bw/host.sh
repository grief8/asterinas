#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

set -e

/usr/local/benchmark/iperf/bin/iperf3 -c 127.0.0.1 -f m
pgrep qemu | xargs kill
#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

set -e

cd /test/runc-hello/
/test/runc --debug run mycontainer
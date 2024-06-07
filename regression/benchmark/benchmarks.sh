#!/bin/bash

# SPDX-License-Identifier: MPL-2.0
set -e

# Define the directory where benchmark results and templates are stored
BENCHMARK_DIR="$(dirname "$0")"

prepare_benchmark() {
    mkdir -p "/benchmark"
    # Copy sysbench 
    cp /usr/local/benchmark/sysbench/bin/sysbench /benchmark
    # Copy getpid
    pushd "${BENCHMARK_DIR}/../app/getpid"
    gcc getpid.c -o /benchmark/getpid
    popd
}

# Common functions for running benchmarks and processing results
run_benchmark() {
    local benchmark=$1
    local avg_pattern="$2"
    local avg_field="$3"
    local os="$4"
    local output="${BENCHMARK_DIR}/output.txt"
    local result_template="${BENCHMARK_DIR}/result_template_${benchmark}.json"
    local result_file="result_${benchmark}_${os}.json"
    local bench_script="${BENCHMARK_DIR}/${benchmark}.sh"

    # Run the benchmark in Linux/Asterinas and save the output to a file
    if [ "$os" == "linux" ]; then
        bash "${bench_script}" | tee "${output}"
    elif [ "$os" == "asterinas" ]; then
        make run BENCHMARK=${benchmark} ENABLE_KVM=0 | tee "${output}"
    else
        echo "Error: Invalid OS specified"
        return 1
    fi
    
    # Parse the average value from the benchmark output
    AVG=$(grep "${avg_pattern}" "${output}" | awk "{print \$$avg_field}")
    
    if [ -z "$AVG" ]; then
        echo "Error: Failed to parse benchmark results from ${output}"
        return 1
    fi
    
    # Update the result template with the average value
    jq --argjson avg "$AVG" \
        '(.[] | select(.extra == "avg") | .value) |= $avg' \
        "${result_template}" > "${result_file}"
}

# Run the specified benchmark
case "$1" in
    sysbench)
        run_benchmark "sysbench" "avg:" 'NF' "$2"
        ;;
    getpid)
        run_benchmark "getpid" "Syscall average latency:" '4' "$2"
        ;;
    *)
        echo "Error: Invalid benchmark specified"
        exit 1
        ;;
esac

#!/bin/sh
cd $(dirname "$0")

options="\
--threads=4 \
--linux_perf_instr \
--linux_perf_branch \
--max_file_size=32 \
--timeout=1 \
$HFUZZ_RUN_ARGS"

echo "options: $options"

HFUZZ_RUN_ARGS="$options" cargo hfuzz run fuzz --color=always

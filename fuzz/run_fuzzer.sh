#!/bin/sh
HFUZZ_RUN_ARGS="\
--threads=4\
--linux_perf_instr\
--linux_perf_branch\
--max_file_size=32"

cargo hfuzz run fuzz --color=always
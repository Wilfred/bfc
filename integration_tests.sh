#!/bin/bash

BLACK=$(tput setaf 0)
BLUE=$(tput setaf 1)
GREEN=$(tput setaf 2)
CYAN=$(tput setaf 3)
RED=$(tput setaf 4)
MAGENTA=$(tput setaf 5)
YELLOW=$(tput setaf 6)
WHITE=$(tput setaf 7)

BOLD=$(tput bold)
RESET=$(tput sgr0)

function summary {
    echo -e "$BOLD$GREEN==>$WHITE ${1}$RESET"
}

failed=0

function compile_and_run {
    local test_program=$1

    # Compile the file.
    ./target/release/bfc sample_programs/$test_program
    if [[ $? -ne 0 ]]; then
        echo "Compilation failed!"
        failed=1
        return
    fi

    # Run it, saving output.
    local executable="${test_program%.*}"
    local input=sample_programs/${test_program}.in

    if [ -f $input ]; then
        ./$executable < $input > output.txt
    else
        ./$executable > output.txt
    fi
    if [[ $? -ne 0 ]]; then
        echo "Program crashed!"
        failed=1
        return
    fi

    # Compare output.
    local expected_output=sample_programs/${test_program}.out
    if [ -f $expected_output ]; then
        diff output.txt $expected_output > /dev/null
        if [[ $? -ne 0 ]]; then
            echo "Output differs!"
            failed=1
            return
        fi
    fi
}

function check_program {
    summary "Testing $1"
    compile_and_run $1

    # Cleanup.
    rm -f ${1%.*} output.txt
}

check_program bangbang.bf
check_program hello_world.bf
check_program bottles.bf
check_program factor.bf
check_program mandelbrot.bf
check_program life.bf

exit $failed

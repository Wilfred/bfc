#!/bin/bash

# Die on first error.
set -e

function compile_and_run {
    local test_program=$1
    echo -n "Testing $test_program"
    
    # Compile the file.
    ./target/release/bfc sample_programs/$test_program

    # Run it, saving output.
    local executable="${test_program%.*}"
    local input=sample_programs/${test_program}.in

    if [ -f $input ]; then
        ./$executable < $input > output.txt
    else
        ./$executable > output.txt
    fi

    # Compare output.
    local expected_output=sample_programs/${test_program}.out
    if [ -f $expected_output ]; then
        echo -n " (checked output)"
        diff output.txt $expected_output
    fi

    # Cleanup.
    rm $executable output.txt

    echo
}

compile_and_run bangbang.bf
compile_and_run hello_world.bf
compile_and_run bottles.bf
compile_and_run factor.bf
compile_and_run mandelbrot.bf
compile_and_run life.bf

# An optimising compiler for BF

[![Build Status](https://travis-ci.org/Wilfred/Bfc.svg?branch=master)](https://travis-ci.org/Wilfred/Bfc)

GPLv2 or later license.

## LLVM

The LLVM implementation is a C++ program that compiles BF to LLVM
IR. It has been written on a 32-bit x86 machine and may not work
elsewhere.

You will need LLVM and boost installed to compile bfc.

    $ cd llvm
    $ make

## Test programs

http://www.hevanet.com/cristofd/brainfuck/tests.b is a treasure trove
of implementation tests. Most implementations here don't pass all
these tests yet.

## Future Work

Optimise it! See:

* https://code.google.com/p/esotope-bfc/wiki/Optimization
* http://calmerthanyouare.org/2015/01/07/optimizing-brainfuck.html
* http://2π.com/10/brainfuck-using-llvm

See also the compiler.md file in this repo.

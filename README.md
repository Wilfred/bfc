# An optimising compiler for BF

[![Build Status](https://travis-ci.org/Wilfred/Bfc.svg?branch=master)](https://travis-ci.org/Wilfred/Bfc)

GPLv2 or later license.

## LLVM

The LLVM implementation is a C++ program that compiles BF to LLVM
IR. It has been written on a 32-bit x86 machine and may not work
elsewhere.

    $ cd llvm
    $ make

## Test programs

http://www.hevanet.com/cristofd/brainfuck/tests.b is a treasure trove
of implementation tests. Most implementations here don't pass all
these tests yet.

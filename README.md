# An optimising compiler for BF

[![Build Status](https://travis-ci.org/Wilfred/bfc.svg?branch=master)](https://travis-ci.org/Wilfred/bfc)

BFC is an optimising compiler for
[BF](https://en.wikipedia.org/wiki/Brainfuck).

It is written in C++ and uses LLVM.

```
BF source -> BFC IR -> LLVM IR -> x86_32 Binary
```

GPLv2 or later license.

## Compiling

You will need LLVM and boost installed to compile bfc.

    $ cd llvm
    $ make

## Usage

```
$ make
$ build/compiler sample_programs/hello_world.bf
$ lli hello_world.ll
Hello World!
```

## Running tests

```
$ make test
```

## Test programs

There are a few test programs in this repo, but
http://www.hevanet.com/cristofd/brainfuck/tests.b is also an excellent
collection of test BF programs.

## Optimisations

Currently, bfc only uses LLVM's optimisations. There's a roadmap in
compiler.md of optimisations we will implement at the BFC IR level.

There are also some interesting other projects for optimising BF
programs:

* https://code.google.com/p/esotope-bfc/wiki/Optimization
* http://calmerthanyouare.org/2015/01/07/optimizing-brainfuck.html
* http://2π.com/10/brainfuck-using-llvm

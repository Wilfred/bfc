# An optimising compiler for BF

[![Build Status](https://travis-ci.org/Wilfred/bfc.svg?branch=master)](https://travis-ci.org/Wilfred/bfc)

BFC is an optimising compiler for
[BF](https://en.wikipedia.org/wiki/Brainfuck).

It is written in C++ and uses LLVM.

```
BF source -> BFC IR -> LLVM IR -> x86_32 Binary
```

GPLv2 or later license.

<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc/generate-toc again -->
**Table of Contents**

- [An optimising compiler for BF](#an-optimising-compiler-for-bf)
    - [Compiling](#compiling)
    - [Usage](#usage)
    - [Running tests](#running-tests)
    - [Test programs](#test-programs)
    - [Optimisations](#optimisations)
        - [Coalescing Increments](#coalescing-increments)
    - [Other projects optimising BF](#other-projects-optimising-bf)

<!-- markdown-toc end -->


## Compiling

You will need LLVM and boost installed to compile bfc.

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

bfc can use LLVM's optimisations, but it also offers some BF-specific
optimisations. There's a roadmap in
[optimisations.md](optimisations.md) of optimisations we will
implement at the BFC IR level.

### Combining Instructions

We combine successive increments/decrements (currently excluding loop bodies).

```
   Compile             Optimise
+++  ->   BFIncrement 1   ->   BFIncrement 3
          BFIncrement 1
          BFIncrement 1
```

If increments/decrements cancel out, we remove them entirely.

```
   Compile              Optimise
+-   ->   BFIncrement  1    ->   # nothing!
          BFIncrement -1
```

We do the same thing for data increments/decrements:

```
   Compile                 Optimise
>>>  ->   BFDataIncrement 1   ->   BFDataIncrement 3
          BFDataIncrement 1
          BFDataIncrement 1

   Compile                  Optimise
><   ->   BFDataIncrement  1    ->   # nothing!
          BFDataIncrement -1
```

## Other projects optimising BF

There are also some interesting other projects for optimising BF
programs:

* https://code.google.com/p/esotope-bfc/wiki/Optimization
* http://calmerthanyouare.org/2015/01/07/optimizing-brainfuck.html
* http://2π.com/10/brainfuck-using-llvm

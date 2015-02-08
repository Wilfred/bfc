# Brainfrack

[![Build Status](https://travis-ci.org/Wilfred/Brainfrack.svg?branch=master)](https://travis-ci.org/Wilfred/Brainfrack)

This repo contains various
[BF](http://en.wikipedia.org/wiki/Brainfuck) implementations, both
interpreters and a compiler. I use BF as a coding exercise to explore
unfamiliar programming languages or tools.

GPLv2 or later license.

<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc/generate-toc again -->
**Table of Contents**

- [Brainfrack](#brainfrack)
    - [Java](#java)
        - [Compiling](#compiling)
        - [Usage](#usage)
    - [Haskell](#haskell)
    - [Clojure](#clojure)
    - [C](#c)
    - [LLVM](#llvm)

<!-- markdown-toc end -->

## Java

### Compiling

Apache Maven required.

    $ cd java/brainfrack
    $ mvn package

### Usage

Brainfrack takes programs as command line arguments with an `-i` flag:

    $ java -cp target/brainfrack-0.1.jar com.github.wilfred.App -i "++++++++++[>+++++++>++++++++++>+++>+<<<<-]>++.>+.+++++++..+++.>++.<<+++++++++++++++.>.+++.------.--------.>+.>."
    Hello world!

## Haskell

The Haskell implementation reads programs from standard in.

    $ cd haskell
    $ ghc Brainfrack.hs
    $ cat ../sample_programs/hello_world.bf | ./Brainfrack

## Clojure

The Clojure implementation reads programs from standard in.

    $ cd clojure/brainfrack
    $ lein compile
    $ cat ../../sample_programs/hello_world.bf | lein trampoline run

## C

The C implementation reads programs from standard in.

    $ cd c
    $ make
    $ cat ../sample_programs/hello_world.bf | ./brainfrack

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

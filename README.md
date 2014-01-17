# Brainfrack

A simple [brainf***](http://en.wikipedia.org/wiki/Brainfuck)
(henceforth BF) interpreter in multiple languages. GPLv2 or later
license.

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

(in progress)

    $ lli hello.ll

Or

    $ llc hello.ll
    $ gcc hello.s
    $ ./a.out

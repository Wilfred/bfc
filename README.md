# Brainfrack

A simple brainf*** (henceforth BF) interpreter in multiple
languages. GPLv2 or later license.

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

### Compiling

    $ cd haskell
    $ ghc Brainfrack.hs
    
### Usage

The Haskell implementation reads programs from standard in.

    $ cd haskell
    $ cat ../sample_programs/hello_world.bf | ./Brainfrack

## Clojure

The Clojure implementation reads programs from standard in, but
currently only evaluates the first line.

    $ cd clojure/brainfrack
    $ lein compile
    $ cat ../../sample_programs/hello_world.bf | tr '\n' ' ' | lein trampoline run

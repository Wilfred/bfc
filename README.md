# Brainfrack

A simple brainf*** (henceforth BF) interpreter in Java. GPLv2 or later
license.

## Compiling

Apache Maven required.

    $ cd java/brainfrack
    $ mvn package

## Usage

Brainfrack takes programs as command line arguments with an `-i` flag:

    $ java -cp target/brainfrack-0.1.jar com.github.wilfred.App -i "++++++++++[>+++++++>++++++++++>+++>+<<<<-]>++.>+.+++++++..+++.>++.<<+++++++++++++++.>.+++.------.--------.>+.>."
    Hello world!

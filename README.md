# An optimising compiler for BF

[![Build Status](https://travis-ci.org/Wilfred/bfc.svg?branch=master)](https://travis-ci.org/Wilfred/bfc)

bfc is an optimising compiler for
[BF](https://en.wikipedia.org/wiki/Brainfuck).

It is written in Rust and uses LLVM.

```
BF source -> BF IR -> LLVM IR -> x86_32 Binary
```

GPLv2 or later license. Sample programs are largely written by other
authors and are under other licenses.

<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc/generate-toc again -->
**Table of Contents**

- [An optimising compiler for BF](#an-optimising-compiler-for-bf)
    - [Compiling](#compiling)
    - [Usage](#usage)
    - [Running tests](#running-tests)
    - [Portability](#portability)
    - [Test programs](#test-programs)
    - [Peephole optimisations](#peephole-optimisations)
        - [Combining Instructions](#combining-instructions)
        - [Loop Simplification](#loop-simplification)
        - [Dead Code Elimination](#dead-code-elimination)
    - [Cell Bounds Analysis](#cell-bounds-analysis)
    - [Speculative Execution](#speculative-execution)
    - [Other projects optimising BF](#other-projects-optimising-bf)

<!-- markdown-toc end -->

## Compiling

You will need LLVM and Rust beta installed to compile bfc.

    $ cargo build --release

Debug builds work, but large programs will take a large amount of time
in speculative execution if bfc is compiled without optimisations. You
can disable this by passing `--opt=0` or `--opt=1`.

## Usage

```
$ target/release/bfc sample_programs/hello_world.bf
$ ./hello_world
Hello World!
```

## Running tests

```
$ cargo test
```

## Portability

bfc assumes a word size of 32 bits, so you may get compilation errors
on 64-bit environments.

bfc considers cells to be single bytes, and arithmetic wraps
around. As a result, `-` sets cell #0 to 255.

bfc provides 30,000 cells. Accessing cells outside of this range is
explicitly undefined, and will probably segfault your program. This is
not guaranteed: your program may terminate normally (e.g. `<-` will be
optimised away rather than crashing).

bfc requires brackets to be balanced, so `+[]]` is rejected.

## Test programs

There are a few test programs in this repo, but
http://www.hevanet.com/cristofd/brainfuck/tests.b is an excellent
collection of small test BF programs and some more elaborate
programs can be found at
[1](http://esoteric.sange.fi/brainfuck/bf-source/prog/) and
[2](http://www.hevanet.com/cristofd/brainfuck/).

## Peephole optimisations

bfc provides a range of peephole optimisations. We use quickcheck to
ensure our optimisations are in the optimal order (by verifying that
our optimiser is idempotent).

There's also a roadmap in [optimisations.md](optimisations.md) of
optimisations we haven't yet implemented.

### Combining Instructions

We combine successive increments/decrements:

```
   Compile            Combine
+++  =>   Increment 1   =>   Increment 3
          Increment 1
          Increment 1
```

If increments/decrements cancel out, we remove them entirely.

```
   Compile             Combine
+-   =>   Increment  1    =>   # nothing!
          Increment -1
```

We do the same thing for data increments/decrements:

```
   Compile                Combine
>>>  =>   DataIncrement 1   =>   DataIncrement 3
          DataIncrement 1
          DataIncrement 1

   Compile                 Combine
><   =>   DataIncrement  1    =>   # nothing!
          DataIncrement -1
```

We do the same thing for successive sets:

```
      Combine
Set 1   =>   Set 2
Set 2

```

We combine sets and increments too:

```
  Compile            Known zero:         Combine
+   =>   Increment 1   =>   Set 0      =>   Set 1
                              Increment 1

```

We remove increments when there's a set immediately after:

```
            Combine
Increment 1   =>   Set 2
Set 2

```

We remove both increments and sets if there's a read immediately
after:

```
            Combine
Increment 1   =>   Read
Read

```

### Loop Simplification

`[-]` is a common BF idiom for zeroing cells. We replace that with
`Set`, enabling further instruction combination.

```
   Compile              Simplify
[-]  =>   Loop             =>   Set 0
            Increment -1
```

### Dead Code Elimination

We remove loops that we know are dead.

For example, loops at the beginning of a program:

```
    Compile                  Known zero               DCE
[>]   =>    Loop                 =>     Set 0          => Set 0
              DataIncrement 1           Loop
                                            DataIncrement 
```


Loops following another loop (one BF technique for comments is
`[-][this, is+a comment.]`).

```
      Compile                 Annotate                 DCE
[>][>]   =>  Loop                =>   Loop              =>   Loop
               DataIncrement 1          DataIncrement 1        DataIncrement 1
             Loop                     Set 0                  Set 0
               DataIncrement 1        Loop
                                          DataIncrement 1
```

We remove redundant set commands after loops (often generated by loop
annotation as above).

```
       Remove redundant set
Loop           =>   Loop
  Increment -1        Increment -1
Set 0

```

We also remove dead code at the end of a program.

```
        Remove pure code
Write         =>           Write
Increment 1
```

## Cell Bounds Analysis

BF programs can use up to 30,000 cells, all of which must be
zero-initialised. However, most programs don't use the whole range.

bfc uses static analysis to work out how many cells a BF program may
use, so it doesn't need to allocate or zero-initialise more memory
than necessary.

```
>><< only uses three cells
```

```
[>><<] uses three cells at most
```

```
[>><<]>>> uses four cells at most
```

```
[>] may use any number of cells, so we must assume 30,000
```

## Speculative Execution

bfc executes as much as it can at compile time. For some programs
(such as hello_world.bf) this optimises away the entire program to
just writing to stdout.

For example, `+.` is compiled to simply `putchar(1);` without needing
any cell storage at all.

bfc sets a maximum number of execution steps, avoiding infinite loops
hanging the compiler. As a result `+[]` will have `+` executed (so our
initial cell value is `1` and `[]` will be in the compiled output.

If a program reads from stdin, speculation execution stops. As a
result, `>,` will have `>` executed (setting the initial cell pointer
to 1) and `,` will be in the compiled output.

bfc will either execute loops entirely, or place them in the compiled
output. For example, consider `+[-]+[+,]`. We can execute `[-]`
entirely, but we cannot execute all of `[+,]` at compile time. The
compiled output does not jump into a loop halfway, instead we execute
`+[-]+` at compile time and all of `[+,]` is in the compiled output.

If bfc manages to execute the entire program, it won't bother
allocating memory for cells:

```
$ cargo run -- sample_programs/hello_world.bf --dump-llvm
@known_outputs = constant [13 x i8] c"Hello World!\0A"

declare i32 @write(i32, i8*, i32)

define i32 @main() {
entry:
  %0 = call i32 @write(i32 0, i8* getelementptr inbounds ([13 x i8]* @known_outputs, i32 0, i32 0), i32 13)
  ret i32 0
}
```

## Other projects optimising BF

There are also some interesting other projects for optimising BF
programs:

* https://code.google.com/p/esotope-bfc/wiki/Optimization
* http://www.nayuki.io/page/optimizing-brainfuck-compiler
* http://mearie.org/projects/esotope/bfc/
* http://calmerthanyouare.org/2015/01/07/optimizing-brainfuck.html
* [http://xn--2-umb.com/10/brainfuck-using-llvm](http://xn--2-umb.com/10/brainfuck-using-llvm)
* https://github.com/stedolan/bf.sed (simple optimisations, but
compiles directly to asm)
* https://github.com/matslina/bfoptimization


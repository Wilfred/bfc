# An optimising compiler for brainfuck

[![Crate version](http://meritbadge.herokuapp.com/bfc)](https://crates.io/crates/bfc)
[![docs](https://docs.rs/bfc/badge.svg)](https://docs.rs/crate/bfc/)
[![Build Status](https://travis-ci.org/Wilfred/bfc.svg?branch=master)](https://travis-ci.org/Wilfred/bfc)
[![Coverage Status](https://coveralls.io/repos/Wilfred/bfc/badge.svg?branch=master&service=github)](https://coveralls.io/github/Wilfred/bfc?branch=master)
[![lines of code](https://tokei.rs/b1/github/wilfred/bfc)](https://github.com/Aaronepower/tokei)

bfc is an industrial grade compiler for
[brainfuck](https://en.wikipedia.org/wiki/Brainfuck). It can:

* compile (and cross-compile) BF programs to executables
* optimise runtime speed
* optimise runtime memory usage
* optimise executable size
* show syntax errors with highlighting of the offending source code
* show warnings with highlighting of the offending source code

It is structured as follows:

```
BF source -> BF IR -> LLVM IR -> x86-64 Binary
```

Interested readers may enjoy my blog posts:

* [An Optimising BF Compiler](http://www.wilfred.me.uk/blog/2015/08/29/an-optimising-bf-compiler/)
* [Even More BF Optimisations](http://www.wilfred.me.uk/blog/2015/10/18/even-more-bf-optimisations/)
* [An Industrial Grade BF Compiler](http://www.wilfred.me.uk/blog/2016/02/07/an-industrial-grade-bf-compiler)

<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc-generate-toc again -->
**Table of Contents**

- [An optimising compiler for BF](#an-optimising-compiler-for-bf)
    - [Usage](#usage)
        - [LLVM Version](#llvm-version)
        - [Running tests](#running-tests)
        - [Portability](#portability)
        - [Test programs](#test-programs)
    - [Diagnostics](#diagnostics)
    - [Optimisations](#optimisations)
        - [Peephole optimisations](#peephole-optimisations)
            - [Combining Instructions](#combining-instructions)
            - [Loop Simplification](#loop-simplification)
            - [Dead Code Elimination](#dead-code-elimination)
            - [Reorder with offsets](#reorder-with-offsets)
            - [Multiply-move loops](#multiply-move-loops)
        - [Cell Bounds Analysis](#cell-bounds-analysis)
        - [Speculative Execution](#speculative-execution)
            - [Infinite Loops](#infinite-loops)
            - [Runtime Values](#runtime-values)
            - [Loop Execution](#loop-execution)
    - [License](#license)
    - [Other projects optimising BF](#other-projects-optimising-bf)

<!-- markdown-toc end -->

## Usage

You will need LLVM and Rust installed to compile bfc.

    $ cargo build --release

You can then compile and run BF programs as follows:

```
$ target/release/bfc sample_programs/hello_world.bf
$ ./hello_world
Hello World!
```

You can use debug builds of bfc, but bfc will run much slower on large
BF programs. This is due to bfc's speculative exectuion. You can
disable this by passing `--opt=0` or `--opt=1` when running bfc.

```
$ target/debug/bfc --opt=0 sample_programs/hello_world.bf
```

By default, bfc compiles programs to executables that run on the
current machine. You can explicitly specify architecture using LLVM
target triples:

```
$ target/release/bfc sample_programs/hello_world.bf --target=x86_64-pc-linux-gnu
```

### LLVM Version

LLVM 3.8+ is recommended, as
[there are known bugs with 3.7](https://github.com/Wilfred/bfc/issues/8). Either
download a prebuilt LLVM, or build it as follows:

```
$ wget http://llvm.org/pre-releases/3.8.0/rc1/llvm-3.8.0rc1.src.tar.xz
$ tar -xf llvm-3.8.0rc1.src.tar.xz

$ mkdir -p ~/tmp/llvm_3_8_build
$ cd ~/tmp/llvm_3_8_build

$ cmake -G Ninja /path/to/untarred/llvm
$ ninja
```

bfc depends on llvm-sys, which compiles against whichever
`llvm-config` it finds.

```
$ export PATH=~/tmp/llvm_3_8_build:$PATH
$ cargo build --release
```

### Running tests

```
$ cargo test
```

### Portability

bfc considers cells to be single bytes, and arithmetic wraps
around. As a result, `-` sets cell #0 to 255.

bfc provides 100,000 cells. Accessing cells outside of this range is
explicitly undefined, and will probably segfault your program. bfc
will generate a warning if it can statically prove out-of-range cell
access.

bfc requires brackets to be balanced, so `+[]]` is rejected, unlike
some BF interpreters.

Finally, bfc assumes input files are valid UTF-8.

### Test programs

There are a few test programs in this repo, but
http://www.hevanet.com/cristofd/brainfuck/tests.b is an excellent
collection of small test BF programs and some more elaborate
programs can be found at
[1](http://esoteric.sange.fi/brainfuck/bf-source/prog/) and
[2](http://www.hevanet.com/cristofd/brainfuck/).

## Diagnostics

bfc can report syntax errors and warnings with relevant line numbers
and highlighting.

![diagnostics screenshot](images/bfc_diagnostics.png)

Note that some warning are produced during optimisation, so disabling
optimisations will reduce warnings.

## Optimisations

### Peephole optimisations

bfc provides a range of peephole optimisations. We use quickcheck to
ensure our optimisations are in the optimal order (by verifying that
our optimiser is idempotent).

#### Combining Instructions

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

We combine pointer increments:

```
   Compile            Combine
+++  =>   PointerIncrement 1   =>   PointerIncrement 2
          PointerIncrement 1
```

We do the same thing for successive sets:

```
      Combine
Set 1   =>   Set 2
Set 2

```

We combine sets and increments too:

```
  Compile            Known zero       Combine
+   =>   Increment 1   =>   Set 0       =>   Set 1
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

We track the current cell position in straight-line code. If we can
determine the last instruction to modify the current cell, it doesn't
need to be immediately previous. For example, `+>-<,`:

```
                   Combine
Increment 1          =>   PointerIncrement 1
PointerIncrement 1        Increment -1
Increment -1              PointerIncrement -1
PointerIncrement -1       Read
Read

```

#### Loop Simplification

`[-]` is a common BF idiom for zeroing cells. We replace that with
`Set`, enabling further instruction combination.

```
   Compile              Simplify
[-]  =>   Loop             =>   Set 0
            Increment -1
```

#### Dead Code Elimination

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

Loops where the cell has previously been set to zero:

```
        Compile               Simplify                 DCE
[-]>+<[]  =>   Loop              =>    Set 0            =>  Set 0
                 Increment -1          DataIncrement 1      DataIncrement 1
               DataIncrement 1         Increment 1          Increment 1
               Increment 1             DataIncrement -1     DataIncrement -1
               DataIncrement -1        Loop
               Loop
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

Finally, we remove cell modifications that are immediately overwritten
by reads, e.g. `+,` is equivalent to `,`.

#### Reorder with offsets

Given a sequence of instructions without loops or I/O, we can safely
reorder them to have the same effect (we assume no out-of-bound cell
access).

This enables us to combine pointer operations:

```
    Compile                   Reorder
>+>   =>   PointerIncrement 1   =>    Increment 1 (offset 1)
           Increment 1                PointerIncrement 2
           PointerIncrement 1
```

We also ensure we modify cells in a consistent order, to aid cache
locality. For example, `>+<+>>+` writes to cell #1, then cell #0, then
cell #2. We reorder these instructions to obtain:

```
Increment 1 (offset 0)
Increment 1 (offset 1)
Increment 1 (offset 2)
PointerIncrement 2
```

#### Multiply-move loops

bfc can detect loops that perform multiplication and converts them to
multiply instructions. This works for simple cases like `[->++<]`
(multiply by two into the next cell) as well as more complex cases
like `[>-<->>+++<<]`.

### Cell Bounds Analysis

BF programs can use up to 100,000 cells, all of which must be
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
[>] may use any number of cells, so we must assume 100,000
```

### Speculative Execution

bfc executes as much as it can at compile time. For some programs
(such as hello_world.bf) this optimises away the entire program to
just writing to stdout. bfc doesn't even need to allocate memory for
cells in this situation.

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

#### Infinite Loops

bfc sets a maximum number of execution steps, avoiding infinite loops
hanging the compiler. As a result `+[]` will have `+` executed (so our
initial cell value is `1` and `[]` will be in the compiled output.

#### Runtime Values

If a program reads from stdin, speculation execution stops. As a
result, `>,` will have `>` executed (setting the initial cell pointer
to 1) and `,` will be in the compiled output.

#### Loop Execution

If loops can be entirely executed at compile time, they will be
removed from the resulting binary. Partially executed loops will be
included in the output, but runtime execution can begin at an
arbitrary position in the loop.

For example, consider `+[-]+[+,]`. We can execute `+[-]+`
entirely, but `[+,]` depends on runtime values. The
compiled output contains `[+,]`, but we start execution at the
`,` (continuing execution from where compile time execution had to
stop).

## License

GPLv2 or later license. Sample programs are largely written by other
authors and are under other licenses.

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
* [Platonic Ideal Brainfuck Interpeter](http://catseye.tc/node/pibfi)
  [(src)](https://github.com/catseye/pibfi) (even has a profiler!)
* https://github.com/rmmh/beefit - using LuaJIT

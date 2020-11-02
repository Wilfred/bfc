---
id: getting-started
title: Getting Started
sidebar_label: Getting Started
---

## Prerequisites

You will need LLVM and Rust installed to compile bfc.

### LLVM Version

LLVM 8 is recommended. Either download a prebuilt LLVM, or build it as
follows:

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

## Compiling bfc

    $ cargo build --release
    
## Running bfc

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


## Diagnostics

bfc can report syntax errors and warnings with relevant line numbers
and highlighting.

![diagnostics screenshot](images/bfc_diagnostics.png)

Note that some warning are produced during optimisation, so disabling
optimisations will reduce warnings.

### Running tests

```
$ cargo test
```

bfc uses quickcheck to ensure optimisations are in the optimal order
(by verifying that our optimiser is idempotent).

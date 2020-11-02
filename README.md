# An optimising compiler for brainfuck

[![Crate version](http://meritbadge.herokuapp.com/bfc)](https://crates.io/crates/bfc)
[![docs](https://docs.rs/bfc/badge.svg)](https://docs.rs/crate/bfc/)
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

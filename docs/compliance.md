---
id: compliance
title: BF Compliance
---

BF is an [underspecified
language](https://en.wikipedia.org/w/index.php?title=Brainfuck&oldid=1139432166#Portability_issues). Most
BF programs just work, but this page documents implementation
decisions.

## Cell Size

bfc considers cells to be single bytes, and arithmetic wraps
around. As a result, `-` sets cell #0 to 255.

## Array Size

bfc provides 100,000 cells. Accessing cells outside of the range #0 to
#99,999 is explicitly undefined. It will
probably segfault.

bfc will generate a warning if it can statically prove out-of-range
cell access.

## Brackets

bfc requires brackets to be balanced. `+[]]` is rejected with a syntax
error, unlike some BF interpreters.

## Source Code

bfc assumes input files are valid UTF-8.

## Sample Programs

Daniel B Cristofani has [an excellent selection of BF
programs](http://www.hevanet.com/cristofd/brainfuck/), including
[several programs explicitly testing implementation
robustness](http://www.hevanet.com/cristofd/brainfuck/tests.b).

[The Brainfuck Archive](http://esoteric.sange.fi/brainfuck/) also
includes a large range of interesting BF programs.

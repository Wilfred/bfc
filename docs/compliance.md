---
id: compliance
title: Compliance
sidebar_label: Compliance
---

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


---
id: faq
title: FAQ
---

## What is bfc?

bfc is an LLVM-based compiler that converts
[Brainfuck](https://en.wikipedia.org/wiki/Brainfuck) to native code.

## Is this a serious project?

It's a legitimate BF implementation with genuine compiler
optimisations. bfc is [referenced on
Wikipedia](https://en.wikipedia.org/wiki/Brainfuck#Implementations)
and [the llvm-sys
docs](https://gitlab.com/taricorp/llvm-sys.rs#documentation).

bfc is also a toy. I'm always pleasantly surprised when people find
actual uses for it.

## How fast is it?

I believe it's the second fastest BF compiler. Mats Linander's [BF
optimisation research
project](https://github.com/matslina/bfoptimization) is still faster
on many programs.

## Why did you do this?

BF is a really good language for learning about compilers. With only
eight operations, it's way more feasible to build elaborate features
like speculative execution and instruction reordering. Also, it's fun.

## What license is bfc under?

bfc is licensed under GPLv2 or later. Documentation (files ending .md
or .png) is licensed under [CC-BY
4.0](https://creativecommons.org/licenses/by/4.0/), and [Twemoji
images](https://twemoji.twitter.com/) are also CC-BY 4.0.

Sample programs are largely written by other authors and are under
other licenses.

## What is the logo?

It's a gingerbread man with **B**u**F**falo plaid. I felt it captured the
silliness of the whole project.

## Why not write "Brainf***"?

The unfortunate name of brainfuck is [discussed on the esolangs
wiki](https://esolangs.org/wiki/Brainfuck). Some writers even
use the term "the unmentionable programming language".

To avoid confusion (and help search engines), I use the term
"brainfuck" on introductions, such as the home page. In other cases I
use "BF".

## Are there other BF optimisers?

Many! Some noteworthy examples I've seen are:

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
* https://github.com/rmmh/beefit (uses LuaJIT)

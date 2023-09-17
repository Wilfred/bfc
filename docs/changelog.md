---
id: changelog
title: Changelog
---

# v1.13.0 (unreleased)

# v1.12.0 (released 17th September 2023)

Removed multiply loop optimisation due to soundness issues.

# v1.11.0 (released 17th June 2023)

Fixed a crash on compiling empty programs at low optimisation levels.

The strip argument is now `-s`, consistent with other compilers, and
binaries are no longer stripped by default.

Stripping is now done during linking, rather than shelling out to
`strip`. This is faster and more robust.

Improved dead code elimination after multiply loops, e.g. `[>++<-][-]`
is equivalent to `[>++<-]`.

# v1.10.0 (released 15th June 2023)

Updated to LLVM 13.

Interface:

* Improved diagnostics display on warnings and errors.
* CLI argument parsing now uses the clap library. No arguments have
  changed, but help and argument parsing errors should be clearer.

# v1.9.0 (released 24th October 2020)

Updated to LLVM 10.0.

Bug fixes:

* Fixed an issue with stripping symbols on macOS (#23).
* ANSI colour codes are only output when stdout is a TTY (#15).

## v1.8.0

Updated to LLVM 8.0.

Fixed a linking error on recent LLVM versions, and a pass manager
segfault on LLVM 8.

## v1.7.0

Bug fixes:

* Fixed a rare crash on programs with a large number of instructions
  had no effect.
* Fixed a memory issue where programs with a large number of cells
  (which were stored on the stack) were misoptimised and
  segfaulted. Cell storage is now on the heap.

Optimisations:

* Stripping symbols from the output binary can now be controlled with
  `--strip`.
* Re-added a multiply loop optimisation. This was removed in 1.5.0 due to
  soundness bugs.

Usability:

* Added a `--version` CLI argument.

## v1.6.0

Extracting multiply loops was causing a variety of soundness failures
and segfaults. This peephole optimisation has been moved to a branch
until it's more robust.

## v1.5.0

Bug fixes:

* Fixed an optimisation that incorrectly removed instructions when
  both `.` and `,` instructions were present.
* Moved to LLVM 3.8, as LLVM 3.7 misoptimised some programs (see #8).

Usability:

* bfc now reports a helpful error on nonexistent targets.
* Improved wording of the warning message on multiply loops that
  access out-of-bounds cells.
* Added a `--passes` CLI argument to customise which bfc optimisation
  passes are run.

## v1.4.0

Portability:

* bfc now supports cross-compilation, so you can compile for any
  architecture that LLVM supports.

* Fixed an issue compiling bfc on ARM.

Performance:

* LLVM's default optimisation levels are tuned for C. We now run LLVM
  optimisation passes twice, to fully leverage LLVM. Many programs now
  execute in less than half the time.

* We now run optimisations using LLVM's API directly rather than
  shelling out to `opt` and `llc`. This provides a modest improvement
  to compile time.

## v1.3.0

[Announcement blog post](http://www.wilfred.me.uk/blog/2016/02/07/an-industrial-grade-bf-compiler/).

Performance:

* We now specify the data layout and target to LLVM, as recommended
  by the LLVM team. In principle this is faster, but we've seen no
  measurable performance boost.

Compatibility:

* bfc now provides up to 100,000 cells. This has been increased to
  support awib, which requires at least 65,535 cells available.

Bug fixes:

* Fixed a compiler crash due to bounds analysis ignoring offsets.
* Show a more helpful error if `llc`, `clang` or `strip` are not
  available.

Usability:

* bfc now reports errors and warnings with colour-coded diagnostics
  and filenames.
* bfc now generates an error with position on syntax errors.
* bfc now generates a warning with position on dead code.
* bfc now generates a warning with position on code that is guaranteed
  to error at runtime.

## v1.2.0

[Announcement blog post](http://www.wilfred.me.uk/blog/2015/10/18/even-more-bf-optimisations/).

Optimisations:

* Compile time execution is now much smarter. Previously, we had to
finish executing loops in order to skip runtime execution. We can now
partly execute loops at runtime. This is a big help to many programs
with a large outer loop, previously they did not benefit from compile
time exeuction.

Compiler performance:

* `--dump-bf` is now much faster.

Bug fixes:

* In some cases, reorder with offset led to miscompilation
(only seen in mandelbrot.bf).

## v1.1.0

Optimisations:

* New optimisation: reorder with offset. See the readme for more
  details.
* Remove redundant sets, dead loop removal and combine before read are
  now smarter. Previously they required adjacent instructions, but
  they now find the next relevant instruction when there are
  irrelevant intermediate instructions.
* LLVM optimisation level can now be set with `--llvm-opt`.

Bug fixes:

* Fixed an issue with writing to stdout during speculative execution
  (we were writing to stdin instead).

Usability:

* Improved the output of `--help`
* `--dump-bf-ir` has been renamed to `--dump-bf`

## v1.0.0

First release! [Announcement blog post](http://www.wilfred.me.uk/blog/2015/08/29/an-optimising-bf-compiler/).

* Compiles to 32-bit x86 binaries.
* Peephole optimisations
* Cell bounds analysis
* Speculative execution

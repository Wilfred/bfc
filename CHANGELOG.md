# v1.2.0 (unreleased)

Compile time execution is now much smarter. Previously, we had to
finish executing loops in order to skip runtime execution. We can now
partly execute loops at runtime. This is a big help to many programs
with a large outer loop, previously they did not benefit from compile
time exeuction.

Performance improvement: `--dump-bf` is now much faster.

# v1.1.0

New optimisation: reorder with offset!

Minor bug fixes:

* Improved the output of `--help`
* `--dump-bf-ir` has been renamed to `--dump-bf`
* LLVM optimisation level can now be set with `--llvm-opt`.
* Fixed an issue with writing to stdout during speculative execution
  (we were writing to stdin instead).

# v1.0.0

First release!

* Compiles to 32-bit x86 binaries.
* Peephole optimisations
* Cell bounds analysis
* Speculative execution

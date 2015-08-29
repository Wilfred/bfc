# (unreleased)

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

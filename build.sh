#!/bin/bash

set -e

ROOT_DIR=$(cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)

cd "$ROOT_DIR/java/brainfrack"
mvn package -q
cd "$ROOT_DIR"

cd "$ROOT_DIR/haskell"
ghc Brainfrack.hs
cd "$ROOT_DIR"

cd "$ROOT_DIR/c"
make
cd "$ROOT_DIR"

cd "$ROOT_DIR/llvm"
make
cd "$ROOT_DIR"


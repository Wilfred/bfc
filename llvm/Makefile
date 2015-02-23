CC = clang++
CFLAGS = -Wall -Wextra -g
LLVM_CONFIG := $(shell llvm-config --cxxflags --ldflags --system-libs --libs core)
LLVM_OVERRIDE = -O0 -UNDEBUG # annoyingly, llvm-config sets -O3
LIBS_CONFIG = -lboost_filesystem -lboost_system

all: compiler

compiler: compiler.cpp
	$(CC) $(CFLAGS) $< $(LLVM_CONFIG) $(LLVM_OVERRIDE) $(LIBS_CONFIG) -o $@

test: test.cpp
	$(CC) $(CFLAGS) -lgtest $< -o $@

.PHONY: format
format:
	find -name "*.cpp" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler

CC = clang++
CFLAGS = -Wall -Wextra -g
LLVM_CONFIG := $(shell llvm-config --cxxflags --ldflags --system-libs --libs core)
LIBS_CONFIG = -lboost_filesystem -lboost_system

all: compiler

compiler: compiler.cpp
	$(CC) $(CFLAGS) $< $(LLVM_CONFIG) $(LIBS_CONFIG) -o $@

.PHONY: format
format:
	find -name "*.cpp" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler

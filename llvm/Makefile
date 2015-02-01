CC = clang++
CFLAGS = -Wall -Wextra -g
LLVM_CONFIG = `llvm-config --cxxflags --ldflags --system-libs --libs core`

all: compiler

compiler: compiler.cpp
	$(CC) $(CFLAGS) $< $(LLVM_CONFIG) -o $@

.PHONY: format
format:
	find -name "*.cpp" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler

CC = clang++
CFLAGS = -Wall -Wextra -g
LLVM_CONFIG := $(shell llvm-config --cxxflags --ldflags --system-libs --libs core)
LLVM_OVERRIDE = -O0 -UNDEBUG -fexceptions
LIBS_CONFIG = -lboost_filesystem -lboost_system

all: compiler

compiler: compiler.cpp bfir.o
	$(CC) $(CFLAGS) $< bfir.o $(LLVM_CONFIG) $(LLVM_OVERRIDE) $(LIBS_CONFIG) -o $@

bfir.o: bfir.cpp bfir.h
	$(CC) $(CFLAGS) -c $< $(LLVM_CONFIG) $(LLVM_OVERRIDE) $(LIBS_CONFIG)

test: test.cpp
	$(CC) $(CFLAGS) -lgtest $< bfir.o -o $@

.PHONY: format
format:
	find -name "*.cpp" -o -name "*.h" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler bfir.o

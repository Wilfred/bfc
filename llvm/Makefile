CC = clang++
CFLAGS = -Wall -Wextra -g
CXXFLAGS := $(shell llvm-config --cxxflags)
LLVM_LDFLAGS := $(shell llvm-config --ldflags --system-libs --libs core)
LLVM_OVERRIDE = -O0 -UNDEBUG -fexceptions
BOOST_LIBS = -lboost_filesystem -lboost_system

all: compiler

compiler: compiler.cpp bfir.o
	$(CC) $(CFLAGS) $< bfir.o $(CXXFLAGS) $(LLVM_LDFLAGS) $(LLVM_OVERRIDE) $(BOOST_LIBS) -o $@

bfir.o: bfir.cpp bfir.h
	$(CC) $(CFLAGS) -c $< $(CXXFLAGS) $(LLVM_OVERRIDE)

test: test.cpp
	$(CC) $(CFLAGS) -lgtest $< bfir.o -o $@

.PHONY: format
format:
	find -name "*.cpp" -o -name "*.h" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler bfir.o

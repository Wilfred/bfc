CC = clang++
CFLAGS = -Wall -Wextra -g
CXXFLAGS := $(shell llvm-config --cxxflags)
LLVM_LDFLAGS := $(shell llvm-config --ldflags --system-libs --libs core)
LLVM_OVERRIDE = -O0 -UNDEBUG -fexceptions
BOOST_LIBS = -lboost_filesystem -lboost_system
TEST_LIBS = -lgtest

all: compiler

compiler: compiler.cpp bfir.o
	$(CC) $(CFLAGS) $< bfir.o $(CXXFLAGS) $(LLVM_LDFLAGS) $(LLVM_OVERRIDE) $(BOOST_LIBS) -o $@

bfir.o: bfir.cpp bfir.h
	$(CC) $(CFLAGS) -c $< $(CXXFLAGS) $(LLVM_OVERRIDE)

run_tests: run_tests.cpp bfir.o
	$(CC) $(CFLAGS) $< bfir.o $(LLVM_LDFLAGS) $(LLVM_OVERRIDE) $(TEST_LIBS) -o $@

.PHONY: test
test: run_tests
	./run_tests

.PHONY: format
format:
	find -name "*.cpp" -o -name "*.h" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler bfir.o run_tests

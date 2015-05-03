CC = clang++
CFLAGS = -Wall -Wextra -g
CXXFLAGS := $(shell llvm-config --cxxflags)
LLVM_LDFLAGS := $(shell llvm-config --ldflags --system-libs --libs core)
LLVM_OVERRIDE = -O0 -UNDEBUG -fexceptions
BOOST_LIBS = -lboost_filesystem -lboost_system
TEST_LIBS = -lgtest

BUILD_DIR = build
VPATH = src

all: $(BUILD_DIR) $(BUILD_DIR)/compiler

$(BUILD_DIR):
	mkdir $@

$(BUILD_DIR)/compiler: compiler.cpp $(BUILD_DIR)/bfir.o $(BUILD_DIR)/optimisations.o $(BUILD_DIR)/parser.o
	$(CC) $(CFLAGS) $^ $(CXXFLAGS) $(LLVM_LDFLAGS) $(LLVM_OVERRIDE) $(BOOST_LIBS) -o $@

$(BUILD_DIR)/bfir.o: bfir.cpp bfir.hpp
	$(CC) $(CFLAGS) -c $< $(CXXFLAGS) $(LLVM_OVERRIDE) -o $@

$(BUILD_DIR)/parser.o: parser.cpp parser.hpp
	$(CC) $(CFLAGS) -c $< $(CXXFLAGS) $(LLVM_OVERRIDE) -o $@

$(BUILD_DIR)/optimisations.o: optimisations.cpp optimisations.hpp
	$(CC) $(CFLAGS) -c $< $(CXXFLAGS) $(LLVM_OVERRIDE) -o $@

$(BUILD_DIR)/run_tests: run_tests.cpp $(BUILD_DIR)/bfir.o $(BUILD_DIR)/optimisations.o $(BUILD_DIR)/parser.o
	$(CC) $(CFLAGS) $^ $(CXXFLAGS) $(LLVM_LDFLAGS) $(LLVM_OVERRIDE) $(TEST_LIBS) -o $@

.PHONY: test
test: $(BUILD_DIR) $(BUILD_DIR)/run_tests
	./$(BUILD_DIR)/run_tests

.PHONY: format
format:
	find -name "*.cpp" -o -name "*.hpp" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -rf $(BUILD_DIR)

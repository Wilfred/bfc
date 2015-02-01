

all: compiler

compiler:
	clang++ -g compiler.cpp -o compiler

.PHONY: format
format:
	find -name "*.cpp" -type f | xargs clang-format -i

.PHONY: clean
clean:
	rm -f compiler

CC=gcc
CFLAGS=-g -Wall -Wextra -std=c99

default: brainfrack

brainfrack: main.c
	$(CC) $(CFLAGS) main.c -o brainfrack

.PHONY: clean
clean:
	rm -f brainfrack

.PHONY: format
format:
	clang-format -i *.c

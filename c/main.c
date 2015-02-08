#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <alloca.h>
#include <unistd.h>
#include <assert.h>
#include <stdbool.h>

// Given the index of an opening bracket, find the index of the
// matching close bracket.
int find_close_index(char *program, int program_len, int open_index) {
    int depth = 0;

    char c;
    for (int i = open_index + 1; i < program_len; i++) {
        c = *(program + i);

        switch (c) {
        case '[':
            depth++;
            break;
        case ']':
            if (depth == 0) {
                return i;
            } else {
                depth--;
                break;
            }
        default:
            // ignore any other character
            break;
        }
    }

    assert(false && "Program is not well bracketed");
}

// Given the index of a closing bracket, find the index of the
// matching open bracket.
int find_open_index(char *program, int program_len, int close_index) {
    char c;
    for (int i = 0; i < close_index; i++) {
        c = *(program + i);

        if (c == '[') {
            if (find_close_index(program, program_len, i) == close_index) {
                return i;
            }
        }
    }

    assert(false && "Program is not well bracketed");
}

enum {
    NUM_CELLS = 30000
};

void eval_program(char *program) {
    int program_len = strlen(program);

    // Our cells are initialised to 0.
    char cells[NUM_CELLS] = {};
    int data_index = 0;
    int instruction_index = 0;

    char c;
    while (instruction_index < program_len) {
        c = *(program + instruction_index);

        switch (c) {
        case '>':
            data_index++;
            assert(data_index < NUM_CELLS && "Tried to access beyond the last cell");
            instruction_index++;
            break;
        case '<':
            assert(data_index > 0 && "Tried to access a negative cell index");
            data_index--;
            instruction_index++;
            break;
        case '+':
            cells[data_index]++;
            instruction_index++;
            break;
        case '-':
            cells[data_index]--;
            instruction_index++;
            break;
        case '.':
            printf("%c", cells[data_index]);
            instruction_index++;
            break;
        case ',':
            // todo: handle errors from getchar
            cells[data_index] = getchar();
            instruction_index++;
            break;
        case '[':
            if (cells[data_index] > 0) {
                // Step into the bracketed section.
                instruction_index++;
            } else {
                // Step over the bracketed section.
                instruction_index =
                    find_close_index(program, program_len, instruction_index) +
                    1;
            }
            break;
        case ']':
            // Jump to the open bracket.
            instruction_index =
                find_open_index(program, program_len, instruction_index);
            break;
        default:
            // ignore other characters
            instruction_index++;
            break;
        }
    }
}

void *realloc_or_die(void *ptr, size_t size) {
    void *p = realloc(ptr, size);

    if (p == NULL) {
        fprintf(stderr, "Out of memory! Exiting.");
        exit(EXIT_FAILURE);
    }

    return p;
}

char *read_string(int file_descriptor) {
    size_t BUFFER_INCREMENT = sizeof(char) * 1024;
    char *s = malloc(BUFFER_INCREMENT);

    size_t total_bytes_read = 0;

    ssize_t bytes_read;
    while ((bytes_read = read(file_descriptor, s + total_bytes_read,
                              BUFFER_INCREMENT))) {
        if (bytes_read == -1) {
            fprintf(stderr,
                    "Could not read from file descriptor %d, exiting.\n",
                    file_descriptor);
            exit(EXIT_FAILURE);
        }

        total_bytes_read += bytes_read;
        s = realloc_or_die(s, BUFFER_INCREMENT + total_bytes_read);
    }

    s = realloc_or_die(s, total_bytes_read + 1);
    s[total_bytes_read] = '\0';

    return s;
}

static int STDIN_FD = 0;

int main() {
    char *program = read_string(STDIN_FD);
    eval_program(program);
    free(program);

    return 0;
}

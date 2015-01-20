#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <alloca.h>
#include <unistd.h>

// Given the index of an opening bracket, find the index of the
// matching close bracket.
// TODO: Handle malformed programs that aren't well-bracketed.
int find_close_index(char* program, int program_len, int open_index) {
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

    // We are assuming the program is well-bracketed, but we need to
    // return something to keep GCC happy.
    return -1;
}

// Given the index of a closing bracket, find the index of the
// matching open bracket.
// TODO: Handle malformed programs that aren't well-bracketed.
int find_open_index(char* program, int program_len, int close_index) {
    char c;
    for (int i = 0; i < close_index; i++) {
        c = *(program + i);

        if (c == '[') {
            if (find_close_index(program, program_len, i) == close_index) {
                return i;
            }
        }
    }

    // We are assuming the program is well-bracketed, but we need to
    // return something to keep GCC happy.
    return -1;
}

void eval_program(char* program) {
    int program_len = strlen(program);
    
    // Our cells are initialised to 0.
    char cells[30000] = {};
    int data_index = 0;
    int instruction_index = 0;

    char c;
    while (instruction_index < program_len) {
        c = *(program + instruction_index);

        switch (c) {
        case '>':
            data_index++;
            instruction_index++;
            break;
        case '<':
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
                instruction_index = find_close_index(program, program_len, instruction_index) + 1;
            }
            break;
        case ']':
            // Jump to the open bracket.
            instruction_index = find_open_index(program, program_len, instruction_index);
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
        exit(1);
    }

    return p;
}

char *read_string(int file_descriptor) {
    char *s = NULL;
    int total_bytes_read = 0;
    
    int BUFFER_SIZE = sizeof(char) * 1024;
    char *temp_buffer = alloca(BUFFER_SIZE);

    int bytes_read;
    // todo: handle errors from read()
    while((bytes_read = read(file_descriptor, temp_buffer, BUFFER_SIZE))) {
        if (bytes_read == -1) {
            fprintf(stderr, "Could not read from file descriptor %d, exiting.\n", file_descriptor);
            exit(1);
        }
        
        s = realloc_or_die(s, total_bytes_read + bytes_read);
        memcpy(s + total_bytes_read, temp_buffer, bytes_read);
        total_bytes_read += bytes_read;
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
}

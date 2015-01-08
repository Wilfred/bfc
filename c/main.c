#include <stdio.h>
#include <stdlib.h>
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

void eval_program(char* program, int program_len) {
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

int main() {
    // todo: handle programs of arbitrary size
    int MAX_PROGRAM_SIZE = 1024;
    char *program = malloc(sizeof(char) * MAX_PROGRAM_SIZE);

    int STDIN_FD = 0;
    // todo: handle errors from read()
    int program_len = read(STDIN_FD, program, MAX_PROGRAM_SIZE);

    int return_code = 0;
    if (program_len == -1) {
        printf("Could not read from stdin");
        return_code = 1;
    } else {
        eval_program(program, program_len);
    }

    free(program);

    return return_code;
}

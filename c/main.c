#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

// Given the index of an opening bracket, find the index of the
// matching close bracket.
// TODO: Handle malformed programs that aren't well-bracketed.
int find_close(char* program, int program_len, int open_index) {
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
            break;
        case '<':
            data_index--;
            break;
        case '+':
            cells[data_index]++;
            break;
        case '-':
            cells[data_index]--;
            break;
        case '.':
            printf("%c", cells[data_index]);
            break;
        case ',':
            // todo: handle errors from getchar
            cells[data_index] = getchar();
            break;
        case '[':
            // todo
            break;
        case ']':
            // todo
            break;
        default:
            // ignore other characters
            break;
        }

        instruction_index++;
    }
}

int main() {
    // todo: handle programs of arbitrary size
    int MAX_PROGRAM_SIZE = 1024;
    char *program = malloc(sizeof(char) * MAX_PROGRAM_SIZE);

    int STDIN_FD = 0;
    // todo: handle errors from read()
    int program_len = read(STDIN_FD, program, MAX_PROGRAM_SIZE);

    eval_program(program, program_len);

    free(program);

    return 0;
}

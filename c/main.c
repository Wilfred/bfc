#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

void eval_program(char* program, int program_len) {
    // Our cells are initialised to 0.
    char cells[30000] = {};
    int data_index = 0;
    int instruction_index = 0;

    char c;
    while (instruction_index < program_len) {
        c = *(program + instruction_index);

        if (c == '[') {
            printf("%c (open!)\n", c);
        } else {
            printf("%c\n", c);
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

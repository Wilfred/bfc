#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

void eval_program(char* program, int program_len) {
    // Our cells are initialised to 0.
    char cells[30000] = {};
}

int main() {
    // todo: handle programs of arbitrary size
    int MAX_PROGRAM_SIZE = 1024;
    char *program = malloc(sizeof(char) * MAX_PROGRAM_SIZE);

    int STDIN_FD = 0;
    // todo: handle errors from read()
    int program_len = read(STDIN_FD, program, MAX_PROGRAM_SIZE);

    printf("program: %s\n", program);

    free(program);

    return 0;
}

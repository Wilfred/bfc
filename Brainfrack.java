import java.util.Stack;

public class Brainfrack {
    public static void main(String[] args) {
        String printCharacterA = "+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++ .";
        Interpreter.evaluate(printCharacterA);
    }
}

/* TODO: exceptions for stackoverflow, accessing memory beyond MEMORY_SIZE, ] without [, [ without ] */
class Interpreter {
    static final int MEMORY_SIZE = 30000;
    
    private static char[] memory = new char[MEMORY_SIZE];

    private static int instructionPointer = 0;
    private static int dataPointer = 0;

    // When we encounter a [ we keep track of its position here so we
    // can jump back to it.
    private static Stack instructionStack = new Stack();

    public static void evaluate(String program) {
        while (true) {
            char currentInstruction = program.charAt(instructionPointer);

            if (currentInstruction == '[') {
                if (memory[dataPointer] == 0) {

                    // Jump forward to the *matching* `]`.
                    // FIXME: we assume a matching `]` exists.
                    while (true) {
                        int openBrackets = 0;
                        instructionPointer++;

                        currentInstruction = program.charAt(instructionPointer);

                        if (currentInstruction == ']') {
                            if (openBrackets == 0) {
                                // We've found the matching bracket! Hurrah!
                                break;
                            } else {
                                openBrackets--;
                            }
                        } else if (currentInstruction == '[') {
                            openBrackets++;
                        }
                    }

                } else {
                    instructionStack.push(instructionPointer);
                }
                
            } else if (currentInstruction == ']'){
                instructionPointer = (int)instructionStack.pop();
                continue;
                
            } else if (currentInstruction == '>') {
                dataPointer++;
            } else if (currentInstruction == '<') {
                dataPointer--;
            } else if (currentInstruction == '+') {
                memory[dataPointer]++;
            } else if (currentInstruction == '-') {
                memory[dataPointer]--;
            } else if (currentInstruction == '.') {
                System.out.printf("%c", memory[dataPointer]);
            } else {
                // no-op; we ignore any other characters
            }

            instructionPointer++;
            if (program.length() <= instructionPointer) {
                // We've reached the end of the program, terminate.
                break;
            }
        }

        printMemory();
    }

    private static void printMemory() {
        int numCells = 200;
        System.out.printf("First %d cells of memory:\n", numCells);

        System.out.print("[");
        for (int i=0; i < numCells; i++) {
                System.out.printf("%c, ", memory[i]);
        }
        System.out.print("]");
    }

}

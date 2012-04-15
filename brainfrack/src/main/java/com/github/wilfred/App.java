package com.github.wilfred;

import java.util.Stack;

public class App {
    public static void main(String[] args) {
        if (args.length == 2 && args[0].equals("-i")) {
            Interpreter interpreter = new Interpreter();
            interpreter.evaluate(args[1]);
        } else {
            System.out.println("Usage: java Brainfrack -i <program>");
        }
    }
}

/* TODO: exceptions for stackoverflow, accessing memory beyond MEMORY_SIZE, ] without [, [ without ] */
class Interpreter {
    static final int MEMORY_SIZE = 30000;
    
    public byte[] memory;

    private Integer instructionPointer;
    private Integer dataPointer;

    // When we encounter a [ we keep track of its position here so we
    // can jump back to it.
    private Stack<Integer> instructionStack;

    /* Initialise an interpreter with zeroed memory.
     */
    public Interpreter() {
        memory = new byte[MEMORY_SIZE];

        instructionPointer = 0;
        dataPointer = 0;

        instructionStack = new Stack<Integer>();
    }

    public void evaluate(String program) {
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
                instructionPointer = (Integer)instructionStack.pop();
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

    }

    private void printMemory() {
        int numCells = 200;
        System.out.printf("First %d cells of memory:\n", numCells);

        System.out.print("[");
        for (int i=0; i < numCells; i++) {
                System.out.printf("%c, ", memory[i]);
        }
        System.out.print("]\n");
    }

}

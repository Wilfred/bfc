package com.github.wilfred;

import junit.framework.Test;
import junit.framework.TestCase;
import junit.framework.TestSuite;

public class AppTest extends TestCase {
    public AppTest(String testName) {
        super(testName);
    }

    public static Test suite() {
        return new TestSuite(AppTest.class);
    }

    public void testDataIncrement() {
        Interpreter interpreter = new Interpreter();
        interpreter.evaluate("+");
        assertEquals(1, interpreter.memory[0]);
    }

    public void testDataDecrement() {
        Interpreter interpreter = new Interpreter();
        interpreter.evaluate("-");
        assertEquals(-1, interpreter.memory[0]);
    }

    public void testDataIncrement() {
        Interpreter interpreter = new Interpreter();
        interpreter.evaluate("+");
        assertEquals(1, interpreter.memory[0]);
    }

    public void testComment() {
        Interpreter interpreter = new Interpreter();
        interpreter.evaluate("I'm a comment!");
        assertTrue(interpreter.memory[0] == 0);
    }
}

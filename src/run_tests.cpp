#include "gtest/gtest.h"

#include "bfir.h"

TEST(Instructions, SameInstructionEqual) {
    BFSequence TestProgram;

    BFRead Instruction1;
    BFRead Instruction2;

    ASSERT_EQ(Instruction1, Instruction2);
}

TEST(Instructions, DifferentInstructionNotEqual) {
    BFSequence TestProgram;

    BFRead Instruction1;
    BFWrite Instruction2;

    ASSERT_NE(Instruction1, Instruction2);
}

// todo: link to
// https://code.google.com/p/googletest/source/browse/trunk/src/gtest_main.cc
// insted.
GTEST_API_ int main(int argc, char **argv) {
    printf("Running main() from gtest_main.cc\n");
    testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}

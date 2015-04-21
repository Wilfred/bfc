#include "gtest/gtest.h"

#include "bfir.h"

TEST(Instructions, SameInstructionEqual) {
    BFRead Read1;
    BFRead Read2;

    ASSERT_EQ(Read1, Read2);

    BFIncrement Incr1(1);
    BFIncrement Incr2(1);

    ASSERT_EQ(Incr1, Incr2);
}

TEST(Instructions, DifferentInstructionNotEqual) {
    BFRead Instruction1;
    BFWrite Instruction2;

    ASSERT_NE(Instruction1, Instruction2);

    BFIncrement Incr1(1);
    BFIncrement Incr2(2);

    ASSERT_NE(Incr1, Incr2);
}

// todo: link to
// https://code.google.com/p/googletest/source/browse/trunk/src/gtest_main.cc
// insted.
GTEST_API_ int main(int argc, char **argv) {
    printf("Running main() from gtest_main.cc\n");
    testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}

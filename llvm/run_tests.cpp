#include "gtest/gtest.h"

TEST(Category, Name) { EXPECT_EQ(1, 2); }

// todo: link to
// https://code.google.com/p/googletest/source/browse/trunk/src/gtest_main.cc
// insted.
GTEST_API_ int main(int argc, char **argv) {
    printf("Running main() from gtest_main.cc\n");
    testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}

#include "gtest/gtest.h"

#include "parser.hpp"
#include "bfir.hpp"
#include "optimisations.hpp"

TEST(Instructions, ReadEquality) {
    BFRead Read1;
    BFRead Read2;
    EXPECT_EQ(Read1, Read2);

    BFWrite Write1;
    EXPECT_NE(Read1, Write1);
}

TEST(Instructions, WriteEquality) {
    BFWrite Write1;
    BFWrite Write2;
    EXPECT_EQ(Write1, Write2);

    BFIncrement Incr1(1);
    EXPECT_NE(Write1, Incr1);
}

TEST(Instructions, IncrementEquality) {
    BFIncrement Incr1(1);
    BFIncrement Incr2(1);
    EXPECT_EQ(Incr1, Incr2);

    BFIncrement Incr3(2);
    EXPECT_NE(Incr1, Incr3);
}

TEST(Instructions, SetEquality) {
    BFSet Set1(1);
    BFSet Set2(1);
    EXPECT_EQ(Set1, Set2);

    BFSet Set3(2);
    EXPECT_NE(Set1, Set3);
}

TEST(Instructions, DataIncrementEquality) {
    BFDataIncrement Incr1(1);
    BFDataIncrement Incr2(1);
    EXPECT_EQ(Incr1, Incr2);

    BFDataIncrement Incr3(2);
    EXPECT_NE(Incr1, Incr3);
}

TEST(Instructions, LoopEquality) {
    BFInstPtr Ptr(new BFDataIncrement(1));
    BFProgram Seq1;
    Seq1.push_back(Ptr);
    BFLoop Loop1(Seq1);

    BFInstPtr Ptr2(new BFDataIncrement(1));
    BFProgram Seq2;
    Seq2.push_back(Ptr2);
    BFLoop Loop2(Seq2);

    EXPECT_EQ(Loop1, Loop2);

    BFInstPtr Ptr3(new BFDataIncrement(2));
    BFProgram Seq3;
    Seq3.push_back(Ptr3);
    BFLoop Loop3(Seq3);

    EXPECT_NE(Loop1, Loop3);

    BFProgram Seq4;
    BFLoop Loop4(Seq4);

    EXPECT_NE(Loop1, Loop4);
}

TEST(Instructions, SequenceEquality) {
    BFInstPtr Ptr(new BFDataIncrement(1));
    BFProgram Seq1;
    Seq1.push_back(Ptr);

    BFInstPtr Ptr2(new BFDataIncrement(1));
    BFProgram Seq2;
    Seq2.push_back(Ptr2);

    EXPECT_EQ(Seq1, Seq2);

    BFProgram Seq3;

    EXPECT_NE(Seq1, Seq3);

    BFInstPtr Ptr3(new BFDataIncrement(2));
    BFProgram Seq4;
    Seq4.push_back(Ptr3);

    EXPECT_NE(Seq1, Seq4);

    BFProgram Seq5;
    EXPECT_NE(Seq1, Seq5);
}

TEST(Optimisations, CombineIncrements) {
    BFProgram InitialProgram;

    BFInstPtr Ptr(new BFIncrement(1));
    InitialProgram.push_back(Ptr);

    BFInstPtr Ptr2(new BFIncrement(2));
    InitialProgram.push_back(Ptr2);

    BFProgram ExpectedProgram;

    BFInstPtr Ptr3(new BFIncrement(3));
    ExpectedProgram.push_back(Ptr3);

    EXPECT_EQ(ExpectedProgram, combineIncrements(InitialProgram));
}

TEST(Optimisations, CombineAndRemoveIncrements) {
    BFProgram InitialProgram;

    BFInstPtr Ptr(new BFIncrement(1));
    InitialProgram.push_back(Ptr);

    BFInstPtr Ptr2(new BFIncrement(-1));
    InitialProgram.push_back(Ptr2);

    BFInstPtr Ptr3(new BFDataIncrement(1));
    InitialProgram.push_back(Ptr3);

    BFProgram ExpectedProgram;

    BFInstPtr Ptr4(new BFDataIncrement(1));
    ExpectedProgram.push_back(Ptr4);

    EXPECT_EQ(ExpectedProgram, combineIncrements(InitialProgram));
}

TEST(Optimisations, DontCombineDifferentIncrements) {
    std::string InitialProgramSrc = "+>";
    BFProgram InitialProgram = parseSource(InitialProgramSrc);

    EXPECT_EQ(InitialProgram, combineIncrements(InitialProgram));
    EXPECT_EQ(InitialProgram, combineDataIncrements(InitialProgram));
}

TEST(Optimisations, CombineDataIncrements) {
    BFProgram InitialProgram;

    BFInstPtr Ptr(new BFDataIncrement(1));
    InitialProgram.push_back(Ptr);

    BFInstPtr Ptr2(new BFDataIncrement(2));
    InitialProgram.push_back(Ptr2);

    BFProgram ExpectedProgram;

    BFInstPtr Ptr3(new BFDataIncrement(3));
    ExpectedProgram.push_back(Ptr3);

    EXPECT_EQ(ExpectedProgram, combineDataIncrements(InitialProgram));
}

TEST(Optimisations, CombineAndRemoveDataIncrements) {
    std::string InitialProgramSrc = "><>";
    BFProgram InitialProgram = parseSource(InitialProgramSrc);

    std::string ExpectedProgramSrc = ">";
    BFProgram ExpectedProgram = parseSource(ExpectedProgramSrc);

    EXPECT_EQ(ExpectedProgram, combineDataIncrements(InitialProgram));
}

TEST(Optimisations, MarkZeroes) {
    std::string InitialProgramSrc = "+";
    BFProgram InitialProgram = parseSource(InitialProgramSrc);

    BFProgram ExpectedProgram;
    BFInstPtr Ptr1(new BFSet(0));
    ExpectedProgram.push_back(Ptr1);
    BFInstPtr Ptr2(new BFIncrement(1));
    ExpectedProgram.push_back(Ptr2);

    EXPECT_EQ(ExpectedProgram, markKnownZero(InitialProgram));
}

TEST(Optimisations, CombineSetAndIncrement) {
    std::string InitialProgramSrc = "+";
    BFProgram Program = parseSource(InitialProgramSrc);

    BFProgram ExpectedProgram;
    BFInstPtr Ptr1(new BFSet(1));
    ExpectedProgram.push_back(Ptr1);

    Program = markKnownZero(Program);
    Program = combineSetAndIncrements(Program);

    EXPECT_EQ(ExpectedProgram, Program);
}

// todo: link to
// https://code.google.com/p/googletest/source/browse/trunk/src/gtest_main.cc
// insted.
GTEST_API_ int main(int argc, char **argv) {
    testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}

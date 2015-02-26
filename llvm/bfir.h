#ifndef BFIR_HEADER
#define BFIR_HEADER

#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

using namespace llvm;

extern Value *CellsPtr;
extern Value *CellIndexPtr;

const int CELL_SIZE_IN_BYTES = 1;

class BFInstruction {
  public:
    // Append the appropriate instructions to the given basic
    // block. We may also create new basic blocks, return the next
    // basic block we should append to.
    virtual BasicBlock *compile(Module *, Function *, BasicBlock *) = 0;

    virtual ~BFInstruction(){};
};

using BFInstPtr = std::shared_ptr<BFInstruction>;
using BFSequence = std::vector<BFInstPtr>;

class BFIncrement : public BFInstruction {
  private:
    int Amount;

  public:
    BFIncrement() { Amount = 1; }

    BFIncrement(int Amount_) { Amount = Amount_; };
    virtual BasicBlock *compile(Module *, Function *, BasicBlock *BB) {
        auto &Context = getGlobalContext();

        IRBuilder<> Builder(Context);
        Builder.SetInsertPoint(BB);

        Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
        Value *CurrentCellPtr =
            Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

        Value *CellVal = Builder.CreateLoad(CurrentCellPtr, "cell_value");
        auto IncrementAmount =
            ConstantInt::get(Context, APInt(CELL_SIZE_IN_BYTES * 8, Amount));
        Value *NewCellVal =
            Builder.CreateAdd(CellVal, IncrementAmount, "cell_value");

        Builder.CreateStore(NewCellVal, CurrentCellPtr);

        return BB;
    }
};

class BFRead : public BFInstruction {
  public:
    virtual BasicBlock *compile(Module *Mod, Function *, BasicBlock *BB) {
        auto &Context = getGlobalContext();

        IRBuilder<> Builder(Context);
        Builder.SetInsertPoint(BB);

        Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
        Value *CurrentCellPtr =
            Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

        Function *GetChar = Mod->getFunction("getchar");
        Value *InputChar = Builder.CreateCall(GetChar, "input_char");
        Value *InputByte = Builder.CreateTrunc(
            InputChar, Type::getInt8Ty(Context), "input_byte");
        Builder.CreateStore(InputByte, CurrentCellPtr);

        return BB;
    }
};

class BFWrite : public BFInstruction {
  public:
    virtual BasicBlock *compile(Module *Mod, Function *, BasicBlock *BB) {
        auto &Context = getGlobalContext();

        IRBuilder<> Builder(Context);
        Builder.SetInsertPoint(BB);

        Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
        Value *CurrentCellPtr =
            Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

        Value *CellVal = Builder.CreateLoad(CurrentCellPtr, "cell_value");
        Value *CellValAsChar = Builder.CreateSExt(
            CellVal, Type::getInt32Ty(Context), "cell_val_as_char");

        Function *PutChar = Mod->getFunction("putchar");
        Builder.CreateCall(PutChar, CellValAsChar);

        return BB;
    }
};

class BFDataIncrement : public BFInstruction {
  private:
    int Amount;

  public:
    BFDataIncrement() { Amount = 1; }

    BFDataIncrement(int Amount_) { Amount = Amount_; };
    virtual BasicBlock *compile(Module *, Function *, BasicBlock *BB) {
        auto &Context = getGlobalContext();

        IRBuilder<> Builder(Context);
        Builder.SetInsertPoint(BB);

        Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
        auto IncrementAmount = ConstantInt::get(Context, APInt(32, Amount));
        Value *NewCellIndex =
            Builder.CreateAdd(CellIndex, IncrementAmount, "new_cell_index");

        Builder.CreateStore(NewCellIndex, CellIndexPtr);

        return BB;
    }
};

class BFLoop : public BFInstruction {
  private:
    BFSequence LoopBody;

  public:
    BFLoop(BFSequence LoopBody_) { LoopBody = LoopBody_; }

    virtual BasicBlock *compile(Module *Mod, Function *F, BasicBlock *BB) {
        auto &Context = getGlobalContext();
        IRBuilder<> Builder(Context);

        BasicBlock *LoopHeader = BasicBlock::Create(Context, "loop_header", F);

        // We start by entering the loop header from the previous
        // instructions.
        Builder.SetInsertPoint(BB);
        Builder.CreateBr(LoopHeader);

        BasicBlock *LoopBodyBlock = BasicBlock::Create(Context, "loop_body", F);
        BasicBlock *LoopAfter = BasicBlock::Create(Context, "loop_after", F);

        // loop_header:
        //   %current_cell = ...
        //   %current_cell_is_zero = icmp ...
        //   br %current_cell_is_zero, %loop_after, %loop_body
        Builder.SetInsertPoint(LoopHeader);
        Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
        Value *CurrentCellPtr =
            Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");
        Value *CellVal = Builder.CreateLoad(CurrentCellPtr, "cell_value");

        auto Zero = ConstantInt::get(Context, APInt(CELL_SIZE_IN_BYTES * 8, 0));
        Value *CellValIsZero = Builder.CreateICmpEQ(CellVal, Zero);

        Builder.CreateCondBr(CellValIsZero, LoopAfter, LoopBodyBlock);

        for (auto I = LoopBody.begin(), E = LoopBody.end(); I != E; ++I) {
            LoopBodyBlock = (*I)->compile(Mod, F, LoopBodyBlock);
        }

        Builder.SetInsertPoint(LoopBodyBlock);
        Builder.CreateBr(LoopHeader);

        return LoopAfter;
    }
};

BFSequence parseSource(std::string);

Module *compileProgram(BFSequence *);

#endif

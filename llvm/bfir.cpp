#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/raw_ostream.h"

using namespace llvm;

Value *CellsPtr;
Value *CellIndexPtr;

enum { NUM_CELLS = 30000, CELL_SIZE_IN_BYTES = 1 };

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

Function *createMain(Module *Mod) {
    auto &Context = getGlobalContext();

    FunctionType *FuncType =
        FunctionType::get(Type::getInt32Ty(Context), false);

    Function *Func =
        Function::Create(FuncType, Function::ExternalLinkage, "main", Mod);

    return Func;
}

// Set up the cells and return a pointer to the cells as a Value.
void addCellsInit(Module *Mod, BasicBlock *BB) {
    auto &Context = getGlobalContext();

    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(BB);

    // char *cells = calloc(3000);
    Function *Calloc = Mod->getFunction("calloc");
    std::vector<Value *> CallocArgs = {
        ConstantInt::get(Context, APInt(32, NUM_CELLS)),
        ConstantInt::get(Context, APInt(32, CELL_SIZE_IN_BYTES))};
    CellsPtr = Builder.CreateCall(Calloc, CallocArgs, "cells");

    // int cell_index = 0;
    CellIndexPtr =
        Builder.CreateAlloca(Type::getInt32Ty(Context), NULL, "cell_index_ptr");
    auto Zero = ConstantInt::get(Context, APInt(32, 0));
    Builder.CreateStore(Zero, CellIndexPtr);
}

void addCellsCleanup(Module *Mod, BasicBlock *BB) {
    auto &Context = getGlobalContext();
    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(BB);

    // free(cells);
    Function *Free = Mod->getFunction("free");
    Builder.CreateCall(Free, CellsPtr);

    // exit(0);
    auto Zero = ConstantInt::get(Context, APInt(32, 0));
    Builder.CreateRet(Zero);
}

void declareCFunctions(Module *Mod) {
    auto &Context = getGlobalContext();

    std::vector<Type *> CallocArgs = {Type::getInt32Ty(Context),
                                      Type::getInt32Ty(Context)};
    FunctionType *CallocType =
        FunctionType::get(Type::getInt8PtrTy(Context), CallocArgs, false);
    Function::Create(CallocType, Function::ExternalLinkage, "calloc", Mod);

    std::vector<Type *> FreeArgs = {Type::getInt8PtrTy(Context)};
    FunctionType *FreeType =
        FunctionType::get(Type::getVoidTy(Context), FreeArgs, false);
    Function::Create(FreeType, Function::ExternalLinkage, "free", Mod);

    std::vector<Type *> PutCharArgs = {Type::getInt32Ty(Context)};
    FunctionType *PutCharType =
        FunctionType::get(Type::getInt32Ty(Context), PutCharArgs, false);
    Function::Create(PutCharType, Function::ExternalLinkage, "putchar", Mod);

    std::vector<Type *> GetCharArgs;
    FunctionType *GetCharType =
        FunctionType::get(Type::getInt32Ty(Context), GetCharArgs, false);
    Function::Create(GetCharType, Function::ExternalLinkage, "getchar", Mod);
}

Module *compileProgram(BFSequence *Program) {
    auto &Context = getGlobalContext();
    Module *Mod = new Module("brainfrack test", Context);

    declareCFunctions(Mod);

    Function *Func = createMain(Mod);
    BasicBlock *BB = BasicBlock::Create(Context, "entry", Func);

    addCellsInit(Mod, BB);

    for (auto I = Program->begin(), E = Program->end(); I != E; ++I) {
        BB = (*I)->compile(Mod, Func, BB);
    }

    addCellsCleanup(Mod, BB);

    return Mod;
}

// Return the index of the ']' that matches the '[' at OpenIndex, or -1
// if we don't have one.
ssize_t findMatchingClose(std::string Source, size_t OpenIndex) {
    assert((Source[OpenIndex] == '[') &&
           "Looking for ']' but not starting from a '['");

    int OpenCount = 0;

    for (size_t I = OpenIndex; I < Source.length(); ++I) {
        switch (Source[I]) {
        case '[':
            OpenCount++;
            break;
        case ']':
            OpenCount--;
            break;
        }

        if (OpenCount == 0) {
            return I;
        }
    }

    return -1;
}

BFSequence parseSourceBetween(std::string Source, size_t From, size_t To) {
    BFSequence Program;

    size_t I = From;
    while (I < To) {
        switch (Source[I]) {
        case '+': {
            BFInstPtr ptr(new BFIncrement(1));
            Program.push_back(ptr);
            break;
        }
        case '-': {
            BFInstPtr ptr(new BFIncrement(-1));
            Program.push_back(ptr);
            break;
        }
        case '>': {
            BFInstPtr ptr(new BFDataIncrement(1));
            Program.push_back(ptr);
            break;
        }
        case '<': {
            BFInstPtr ptr(new BFDataIncrement(-1));
            Program.push_back(ptr);
            break;
        }
        case ',': {
            BFInstPtr ptr(new BFRead);
            Program.push_back(ptr);
            break;
        }
        case '.': {
            BFInstPtr ptr(new BFWrite);
            Program.push_back(ptr);
            break;
        }
        case '[': {
            ssize_t MatchingCloseIdx = findMatchingClose(Source, I);
            if (MatchingCloseIdx == -1) {
                errs() << "Unmatched '[' at position " << I << "\n";
                // FIXME: this leaks Program, the instructions, and everything
                // in main.
                exit(EXIT_FAILURE);
            }
            BFInstPtr ptr(new BFLoop(
                parseSourceBetween(Source, I + 1, MatchingCloseIdx)));
            Program.push_back(ptr);
            I = MatchingCloseIdx;
            break;
        }
        case ']': {
            // We will have already stepped over the ']' unless our
            // brackets are not well-matched.
            errs() << "Unmatched ']' at position " << I << "\n";
            // FIXME: this leaks Program, the instructions, and everything in
            // main.
            exit(EXIT_FAILURE);
        }
        default:
            // skip comments
            break;
        }

        ++I;
    }

    return Program;
}

BFSequence parseSource(std::string Source) {
    return parseSourceBetween(Source, 0, Source.length());
}

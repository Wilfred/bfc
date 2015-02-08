#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

#include <stdio.h>

using namespace llvm;

Value *CellsPtr;
Value *CellIndexPtr;

enum { NUM_CELLS = 3000, CELL_SIZE_IN_BYTES = 1 };

class BFInstruction {
  public:
    virtual void compile(IRBuilder<> *) = 0;
};

class BFIncrement : public BFInstruction {
  private:
    int Amount;

  public:
    BFIncrement() { Amount = 1; }

    BFIncrement(int Amount_) { Amount = Amount_; };
    virtual void compile(IRBuilder<> *Builder) {
        LLVMContext &Context = getGlobalContext();

        Value *CellIndex = Builder->CreateLoad(CellIndexPtr, "cell_index");
        Value *CurrentCellPtr =
            Builder->CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

        Value *CellVal = Builder->CreateLoad(CurrentCellPtr, "cell_value");
        auto IncrementAmount =
            ConstantInt::get(Context, APInt(CELL_SIZE_IN_BYTES * 8, Amount));
        Value *NewCellVal =
            Builder->CreateAdd(CellVal, IncrementAmount, "cell_value");

        Builder->CreateStore(NewCellVal, CurrentCellPtr);
    }
};

class BFDataIncrement : public BFInstruction {
  private:
    int Amount;

  public:
    BFDataIncrement() { Amount = 1; }

    BFDataIncrement(int Amount_) { Amount = Amount_; };
    virtual void compile(IRBuilder<> *Builder) {
        LLVMContext &Context = getGlobalContext();

        Value *CellIndex = Builder->CreateLoad(CellIndexPtr, "cell_index");
        auto IncrementAmount = ConstantInt::get(Context, APInt(32, Amount));
        Value *NewCellIndex = Builder->CreateAdd(CellIndex, IncrementAmount);

        Builder->CreateStore(NewCellIndex, CellIndexPtr);
    }
};

Function *createMain(Module *Mod) {
    LLVMContext &Context = getGlobalContext();

    FunctionType *FuncType =
        FunctionType::get(Type::getInt32Ty(Context), false);

    Function *Func =
        Function::Create(FuncType, Function::ExternalLinkage, "main", Mod);

    return Func;
}

// Set up the cells and return a pointer to the cells as a Value.
void addCellsInit(IRBuilder<> *Builder, Module *Mod) {
    auto &Context = getGlobalContext();

    // char *cells = calloc(3000);
    Function *Calloc = Mod->getFunction("calloc");
    std::vector<Value *> CallocArgs = {
        ConstantInt::get(Context, APInt(32, NUM_CELLS)),
        ConstantInt::get(Context, APInt(32, CELL_SIZE_IN_BYTES))};
    CellsPtr = Builder->CreateCall(Calloc, CallocArgs, "cells");

    // int cell_index = 0;
    CellIndexPtr = Builder->CreateAlloca(Type::getInt32Ty(Context), NULL,
                                         "cell_index_ptr");
    auto Zero = ConstantInt::get(Context, APInt(32, 0));
    Builder->CreateStore(Zero, CellIndexPtr);
}

void addCellsCleanup(IRBuilder<> *Builder, Module *Mod) {
    auto &Context = getGlobalContext();

    // Return the current cell value as our exit code, for a sanity
    // check.
    Value *CellIndex = Builder->CreateLoad(CellIndexPtr, "cell_index");
    Value *CurrentCellPtr =
        Builder->CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

    Value *CellVal = Builder->CreateLoad(CurrentCellPtr, "cell_value");
    Value *RetVal =
        Builder->CreateZExt(CellVal, Type::getInt32Ty(Context), "exit_code");

    // free(cells);
    Function *Free = Mod->getFunction("free");
    Builder->CreateCall(Free, CellsPtr);

    Builder->CreateRet(RetVal);
}

void declareCFunctions(Module *Mod) {
    LLVMContext &Context = getGlobalContext();

    std::vector<Type *> CallocArgs = {Type::getInt32Ty(Context),
                                      Type::getInt32Ty(Context)};
    FunctionType *CallocType =
        FunctionType::get(Type::getInt8PtrTy(Context), CallocArgs, false);
    Function::Create(CallocType, Function::ExternalLinkage, "calloc", Mod);

    std::vector<Type *> FreeArgs = {Type::getInt8PtrTy(Context)};
    FunctionType *FreeType =
        FunctionType::get(Type::getVoidTy(Context), FreeArgs, false);
    Function::Create(FreeType, Function::ExternalLinkage, "free", Mod);
}

Module *compileProgram(std::vector<BFInstruction *> *Program) {
    auto &Context = getGlobalContext();
    Module *Mod = new Module("brainfrack test", Context);

    declareCFunctions(Mod);

    Function *Func = createMain(Mod);
    BasicBlock *BB = BasicBlock::Create(getGlobalContext(), "entry", Func);

    IRBuilder<> Builder(getGlobalContext());
    Builder.SetInsertPoint(BB);

    addCellsInit(&Builder, Mod);

    for (auto I = Program->begin(), E = Program->end(); I != E; ++I) {
        (*I)->compile(&Builder);
    }

    addCellsCleanup(&Builder, Mod);

    return Mod;
}

int main() {
    BFIncrement Inst;
    BFDataIncrement DataInst;
    BFDataIncrement DataInst2(-1);
    std::vector<BFInstruction *> Program;
    Program.push_back(&Inst);
    Program.push_back(&Inst);
    Program.push_back(&DataInst);
    Program.push_back(&Inst);
    Program.push_back(&DataInst2);

    Module *Mod = compileProgram(&Program);

    // Print the generated code
    Mod->dump();

    delete Mod;

    return 0;
}

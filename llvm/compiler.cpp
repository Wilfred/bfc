#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

#include <stdio.h>

using namespace llvm;

Value *CellsPtr;
Value *CellIndexPtr;

// Append the LLVM IR for '+'
void addIncrement(IRBuilder<> *Builder) {
    LLVMContext &Context = getGlobalContext();

    Value *CellIndex = Builder->CreateLoad(CellIndexPtr, "cell_index");
    Value *CurrentCellPtr =
        Builder->CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

    Value *CellVal = Builder->CreateLoad(CurrentCellPtr, "cell_value");
    auto One = ConstantInt::get(Context, APInt(32, 0));
    Value *NewCellVal = Builder->CreateAdd(CellVal, One, "cell_value");

    Builder->CreateStore(NewCellVal, CurrentCellPtr);
}

Function *createMain(Module *Mod) {
    LLVMContext &Context = getGlobalContext();

    FunctionType *FuncType =
        FunctionType::get(Type::getInt32Ty(Context), false);

    Function *Func =
        Function::Create(FuncType, Function::ExternalLinkage, "main", Mod);

    return Func;
}

enum { NUM_CELLS = 3000, CELL_SIZE_IN_BYTES = 1 };

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
    CellIndexPtr =
        Builder->CreateAlloca(Type::getInt32Ty(Context), NULL, "cell_index_ptr");
    auto Zero = ConstantInt::get(Context, APInt(32, 0));
    Builder->CreateStore(Zero, CellIndexPtr);
}

void addCellsCleanup(IRBuilder<> *Builder, Module *Mod) {
    auto &Context = getGlobalContext();

    // free(cells);
    Function *Free = Mod->getFunction("free");
    Builder->CreateCall(Free, CellsPtr);

    // return 0;
    Value *RetVal = ConstantInt::get(Context, APInt(32, 0));
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

int main() {
    LLVMContext &Context = getGlobalContext();
    Module Mod("brainfrack test", Context);

    declareCFunctions(&Mod);

    Function *Func = createMain(&Mod);
    BasicBlock *BB = BasicBlock::Create(getGlobalContext(), "entry", Func);

    IRBuilder<> Builder(getGlobalContext());
    Builder.SetInsertPoint(BB);

    addCellsInit(&Builder, &Mod);
    addIncrement(&Builder);
    addCellsCleanup(&Builder, &Mod);

    // Print the generated code
    Mod.dump();

    return 0;
}

#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

#include <stdio.h>

using namespace llvm;

// Append the LLVM IR for '+'
void appendIncrement(IRBuilder<> *Builder) {
    // TODO
}

Function *createMain(Module *Mod) {
    LLVMContext &Context = getGlobalContext();

    FunctionType *FuncType =
        FunctionType::get(Type::getInt32Ty(Context), false);

    Function *Func =
        Function::Create(FuncType, Function::ExternalLinkage, "main", Mod);

    return Func;
}

#define NUM_CELLS 3000

void addPrologue(IRBuilder<> *Builder, Module *Mod) {
    Function *Calloc = Mod->getFunction("calloc");
    std::vector<Value *> CallocArgs(
        1, ConstantInt::get(getGlobalContext(), APInt(32, NUM_CELLS)));
    Builder->CreateCall(Calloc, CallocArgs, "cells");
}

void addEpilogue(IRBuilder<> *Builder) {
    Value *RetVal = ConstantInt::get(getGlobalContext(), APInt(32, 0));
    Builder->CreateRet(RetVal);
}

void declareCFunctions(Module *Mod) {
    LLVMContext &Context = getGlobalContext();

    std::vector<Type *> CallocReturnType(1, Type::getInt64Ty(Context));
    FunctionType *CallocType =
        FunctionType::get(Type::getInt8PtrTy(Context), CallocReturnType, false);
    Function::Create(CallocType, Function::ExternalLinkage, "calloc", Mod);
}

int main() {
    LLVMContext &Context = getGlobalContext();
    Module Mod("brainfrack test", Context);

    declareCFunctions(&Mod);

    Function *Func = createMain(&Mod);
    BasicBlock *BB = BasicBlock::Create(getGlobalContext(), "entry", Func);

    IRBuilder<> Builder(getGlobalContext());
    Builder.SetInsertPoint(BB);

    addPrologue(&Builder, &Mod);
    appendIncrement(&Builder);
    addEpilogue(&Builder);

    // Print the generated code
    Mod.dump();

    return 0;
}

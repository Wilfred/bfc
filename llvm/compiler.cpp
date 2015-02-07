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
    auto CallocArg = ConstantInt::get(getGlobalContext(), APInt(32, NUM_CELLS));
    Builder->CreateCall(Calloc, CallocArg, "cells");
}

void addEpilogue(IRBuilder<> *Builder, Module *Mod) {
    auto &Context = getGlobalContext();
    
    // free(cells);
    Function *Free = Mod->getFunction("free");
    auto FreeArg = ConstantInt::get(Context, APInt(32, NUM_CELLS));
    Builder->CreateCall(Free, FreeArg);
    
    // return 0;
    Value *RetVal = ConstantInt::get(Context, APInt(32, 0));
    Builder->CreateRet(RetVal);
}

void declareCFunctions(Module *Mod) {
    LLVMContext &Context = getGlobalContext();

    std::vector<Type *> CallocReturnType(1, Type::getInt64Ty(Context));
    FunctionType *CallocType =
        FunctionType::get(Type::getInt8PtrTy(Context), CallocReturnType, false);
    Function::Create(CallocType, Function::ExternalLinkage, "calloc", Mod);

    std::vector<Type *> FreeReturnType(1, Type::getInt8PtrTy(Context));
    FunctionType *FreeType =
        FunctionType::get(Type::getVoidTy(Context), FreeReturnType, false);
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

    addPrologue(&Builder, &Mod);
    appendIncrement(&Builder);
    addEpilogue(&Builder, &Mod);

    // Print the generated code
    Mod.dump();

    return 0;
}

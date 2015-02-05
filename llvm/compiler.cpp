#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

#include <stdio.h>

using namespace llvm;

// Append the LLVM IR for -'+'
void appendIncrement(IRBuilder<> *Builder) {
    // placeholder, currently just:
    // int main(void) { return 2; }
    Value *RetVal = ConstantInt::get(getGlobalContext(), APInt(32, 2));
    Builder->CreateRet(RetVal);
}

Function *createMain(Module *Mod) {
    FunctionType *FuncType =
        FunctionType::get(Type::getInt32Ty(getGlobalContext()), false);

    return Function::Create(FuncType, Function::ExternalLinkage, "main", Mod);
}

int main() {
    LLVMContext &Context = getGlobalContext();
    Module TheModule("brainfrack test", Context);

    IRBuilder<> Builder(Context);

    Function *Func = createMain(&TheModule);

    BasicBlock *BB = BasicBlock::Create(Context, "entry", Func);
    Builder.SetInsertPoint(BB);

    appendIncrement(&Builder);

    // Print the generated code
    TheModule.dump();

    return 0;
}

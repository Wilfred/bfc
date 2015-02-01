#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

#include <stdio.h>

using namespace llvm;

static IRBuilder<> Builder(getGlobalContext());

int main() {
    // create a simple:
    // int main(void) { return 2; }

    LLVMContext &Context = getGlobalContext();
    Module *TheModule = new Module("brainfrack test", Context);

    
    FunctionType *FT = FunctionType::get(Type::getInt32Ty(getGlobalContext()),
                                         false);

    Function *F =
        Function::Create(FT, Function::ExternalLinkage, "main", TheModule);

    BasicBlock *BB =
        BasicBlock::Create(getGlobalContext(), "entry", F);
    Builder.SetInsertPoint(BB);

    Value *RetVal = ConstantInt::get(getGlobalContext(), APInt(32, 2));

    Builder.CreateRet(RetVal);

    // Print the generated code
    TheModule->dump();
}

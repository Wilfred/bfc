#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Support/raw_os_ostream.h"

#include "bfir.h"

using namespace llvm;

const int NUM_CELLS = 30000;

Value *CellsPtr;
Value *CellIndexPtr;

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

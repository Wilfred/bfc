#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Support/raw_os_ostream.h"

#include "bfir.h"

using namespace llvm;

Value *CellsPtr;
Value *CellIndexPtr;

const int CELL_SIZE_IN_BYTES = 1;

std::ostream &operator<<(std::ostream &os, const BFInstruction &Inst) {
    return Inst.stream_write(os);
}
std::ostream &operator<<(std::ostream &os, const BFLoop &Inst) {
    return Inst.stream_write(os);
}
std::ostream &operator<<(std::ostream &os, const BFDataIncrement &Inst) {
    return Inst.stream_write(os);
}
std::ostream &operator<<(std::ostream &os, const BFIncrement &Inst) {
    return Inst.stream_write(os);
}
std::ostream &operator<<(std::ostream &os, const BFRead &Inst) {
    return Inst.stream_write(os);
}
std::ostream &operator<<(std::ostream &os, const BFWrite &Inst) {
    return Inst.stream_write(os);
}

bool operator==(const BFInstruction &X, const BFInstruction &Y) {
    if (typeid(X) != typeid(Y)) {
        return false;
    }

    try {
        const BFIncrement &IncrX = dynamic_cast<const BFIncrement &>(X);
        const BFIncrement &IncrY = dynamic_cast<const BFIncrement &>(Y);

        return IncrX.Amount == IncrY.Amount;
    } catch (const std::bad_cast &) {
    }

    try {
        const BFDataIncrement &IncrX = dynamic_cast<const BFDataIncrement &>(X);
        const BFDataIncrement &IncrY = dynamic_cast<const BFDataIncrement &>(Y);

        return IncrX.Amount == IncrY.Amount;
    } catch (const std::bad_cast &) {
    }

    try {
        const BFLoop &LoopX = dynamic_cast<const BFLoop &>(X);
        const BFLoop &LoopY = dynamic_cast<const BFLoop &>(Y);

        if (LoopX.LoopBody.size() != LoopY.LoopBody.size()) {
            return false;
        }

        auto XLB = LoopX.LoopBody;
        auto XI = XLB.begin();
        auto XE = XLB.end();

        auto YLB = LoopY.LoopBody;
        auto YI = YLB.begin();

        for (; XI != XE; ++XI, ++YI) {
            if (**XI != **YI) {
                return false;
            }
        }

    } catch (const std::bad_cast &) {
    }

    return true;
}

bool operator!=(const BFInstruction &X, const BFInstruction &Y) {
    return !(X == Y);
}

void BFProgram::push_back(BFInstPtr P) { Instructions.push_back(P); }

std::vector<BFInstPtr>::iterator BFProgram::begin() {
    return Instructions.begin();
}

std::vector<BFInstPtr>::const_iterator BFProgram::begin() const {
    return Instructions.begin();
}

std::vector<BFInstPtr>::iterator BFProgram::end() { return Instructions.end(); }

std::vector<BFInstPtr>::const_iterator BFProgram::end() const {
    return Instructions.end();
}

std::vector<BFInstPtr>::size_type BFProgram::size() const {
    return Instructions.size();
}

std::ostream &operator<<(std::ostream &os, const BFProgram &Prog) {
    os << "BFProgram\n";

    auto Instructions = Prog.Instructions;
    for (BFInstPtr I : Instructions) {
        os << "  " << (*I) << "\n";
    }

    return os;
}

bool operator==(const BFProgram &X, const BFProgram &Y) {
    if (X.size() != Y.size()) {
        return false;
    }

    auto XI = X.begin();
    auto XE = X.end();

    auto YI = Y.begin();

    for (; XI != XE; ++XI, ++YI) {
        if (**XI != **YI) {
            return false;
        }
    }

    return true;
}

bool operator!=(const BFProgram &X, const BFProgram &Y) { return !(X == Y); }

std::ostream &BFIncrement::stream_write(std::ostream &os) const {
    os << "BFIncrement " << Amount;
    return os;
}

BFIncrement::BFIncrement() { Amount = 1; }

BFIncrement::BFIncrement(int amount) { Amount = amount; }

BasicBlock *BFIncrement::compile(Module &, Function &, BasicBlock &BB) {
    auto &Context = getGlobalContext();

    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(&BB);

    Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
    Value *CurrentCellPtr =
        Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

    Value *CellVal = Builder.CreateLoad(CurrentCellPtr, "cell_value");
    auto IncrementAmount =
        ConstantInt::get(Context, APInt(CELL_SIZE_IN_BYTES * 8, Amount));
    Value *NewCellVal =
        Builder.CreateAdd(CellVal, IncrementAmount, "cell_value");

    Builder.CreateStore(NewCellVal, CurrentCellPtr);

    return &BB;
}

std::ostream &BFDataIncrement::stream_write(std::ostream &os) const {
    os << "BFDataIncrement " << Amount;
    return os;
}

BFDataIncrement::BFDataIncrement() { Amount = 1; }
BFDataIncrement::BFDataIncrement(int Amount_) { Amount = Amount_; };

BasicBlock *BFDataIncrement::compile(Module &, Function &, BasicBlock &BB) {
    auto &Context = getGlobalContext();

    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(&BB);

    Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
    auto IncrementAmount = ConstantInt::get(Context, APInt(32, Amount));
    Value *NewCellIndex =
        Builder.CreateAdd(CellIndex, IncrementAmount, "new_cell_index");

    Builder.CreateStore(NewCellIndex, CellIndexPtr);

    return &BB;
}

std::ostream &BFRead::stream_write(std::ostream &os) const {
    os << "BFRead";
    return os;
}

BasicBlock *BFRead::compile(Module &Mod, Function &, BasicBlock &BB) {
    auto &Context = getGlobalContext();

    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(&BB);

    Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
    Value *CurrentCellPtr =
        Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

    Function *GetChar = Mod.getFunction("getchar");
    Value *InputChar = Builder.CreateCall(GetChar, "input_char");
    Value *InputByte =
        Builder.CreateTrunc(InputChar, Type::getInt8Ty(Context), "input_byte");
    Builder.CreateStore(InputByte, CurrentCellPtr);

    return &BB;
}

std::ostream &BFWrite::stream_write(std::ostream &os) const {
    os << "BFWrite";
    return os;
}

BasicBlock *BFWrite::compile(Module &Mod, Function &, BasicBlock &BB) {
    auto &Context = getGlobalContext();

    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(&BB);

    Value *CellIndex = Builder.CreateLoad(CellIndexPtr, "cell_index");
    Value *CurrentCellPtr =
        Builder.CreateGEP(CellsPtr, CellIndex, "current_cell_ptr");

    Value *CellVal = Builder.CreateLoad(CurrentCellPtr, "cell_value");
    Value *CellValAsChar = Builder.CreateSExt(
        CellVal, Type::getInt32Ty(Context), "cell_val_as_char");

    Function *PutChar = Mod.getFunction("putchar");
    Builder.CreateCall(PutChar, CellValAsChar);

    return &BB;
}

std::ostream &BFLoop::stream_write(std::ostream &os) const {
    os << "BFLoop\n";

    auto LB = LoopBody;
    for (auto I = LB.begin(), E = LB.end(); I != E; ++I) {
        os << "  " << (**I) << "\n";
    }

    return os;
}

BFLoop::BFLoop(BFProgram LoopBody_) { LoopBody = LoopBody_; }

BasicBlock *BFLoop::compile(Module &Mod, Function &F, BasicBlock &BB) {
    auto &Context = getGlobalContext();
    IRBuilder<> Builder(Context);

    BasicBlock *LoopHeader = BasicBlock::Create(Context, "loop_header", &F);

    // We start by entering the loop header from the previous
    // instructions.
    Builder.SetInsertPoint(&BB);
    Builder.CreateBr(LoopHeader);

    BasicBlock *LoopBodyBlock = BasicBlock::Create(Context, "loop_body", &F);
    BasicBlock *LoopAfter = BasicBlock::Create(Context, "loop_after", &F);

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
        LoopBodyBlock = (*I)->compile(Mod, F, *LoopBodyBlock);
    }

    Builder.SetInsertPoint(LoopBodyBlock);
    Builder.CreateBr(LoopHeader);

    return LoopAfter;
}

const int NUM_CELLS = 30000;

Function *createMain(Module &Mod) {
    auto &Context = getGlobalContext();

    FunctionType *FuncType =
        FunctionType::get(Type::getInt32Ty(Context), false);

    Function *Func =
        Function::Create(FuncType, Function::ExternalLinkage, "main", &Mod);

    return Func;
}

// Set up the cells and return a pointer to the cells as a Value.
void addCellsInit(Module &Mod, BasicBlock &BB) {
    auto &Context = getGlobalContext();

    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(&BB);

    // char *cells = calloc(3000);
    Function *Calloc = Mod.getFunction("calloc");
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

void addCellsCleanup(Module &Mod, BasicBlock &BB) {
    auto &Context = getGlobalContext();
    IRBuilder<> Builder(Context);
    Builder.SetInsertPoint(&BB);

    // free(cells);
    Function *Free = Mod.getFunction("free");
    Builder.CreateCall(Free, CellsPtr);

    // exit(0);
    auto Zero = ConstantInt::get(Context, APInt(32, 0));
    Builder.CreateRet(Zero);
}

void declareCFunctions(Module &Mod) {
    auto &Context = getGlobalContext();

    std::vector<Type *> CallocArgs = {Type::getInt32Ty(Context),
                                      Type::getInt32Ty(Context)};
    FunctionType *CallocType =
        FunctionType::get(Type::getInt8PtrTy(Context), CallocArgs, false);
    Function::Create(CallocType, Function::ExternalLinkage, "calloc", &Mod);

    std::vector<Type *> FreeArgs = {Type::getInt8PtrTy(Context)};
    FunctionType *FreeType =
        FunctionType::get(Type::getVoidTy(Context), FreeArgs, false);
    Function::Create(FreeType, Function::ExternalLinkage, "free", &Mod);

    std::vector<Type *> PutCharArgs = {Type::getInt32Ty(Context)};
    FunctionType *PutCharType =
        FunctionType::get(Type::getInt32Ty(Context), PutCharArgs, false);
    Function::Create(PutCharType, Function::ExternalLinkage, "putchar", &Mod);

    std::vector<Type *> GetCharArgs;
    FunctionType *GetCharType =
        FunctionType::get(Type::getInt32Ty(Context), GetCharArgs, false);
    Function::Create(GetCharType, Function::ExternalLinkage, "getchar", &Mod);
}

Module *compileProgram(BFProgram &Program) {
    auto &Context = getGlobalContext();
    Module *Mod = new Module("brainfrack test", Context);

    declareCFunctions(*Mod);

    Function *Func = createMain(*Mod);
    BasicBlock *BB = BasicBlock::Create(Context, "entry", Func);

    addCellsInit(*Mod, *BB);

    for (auto I = Program.begin(), E = Program.end(); I != E; ++I) {
        BB = (*I)->compile(*Mod, *Func, *BB);
    }

    addCellsCleanup(*Mod, *BB);

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

BFProgram parseSourceBetween(std::string &Source, size_t From, size_t To) {
    BFProgram Program;

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

BFProgram parseSource(std::string &Source) {
    return parseSourceBetween(Source, 0, Source.length());
}

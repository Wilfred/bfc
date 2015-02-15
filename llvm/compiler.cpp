#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Support/raw_os_ostream.h"

#include <boost/filesystem.hpp>

#include <fstream>
#include <regex>

using namespace llvm;

Value *CellsPtr;
Value *CellIndexPtr;

enum { NUM_CELLS = 3000, CELL_SIZE_IN_BYTES = 1 };

class BFInstruction {
  public:
    // Append the appropriate instructions to the given basic
    // block. We may also create new basic blocks, return the next
    // basic block we should append to.
    virtual BasicBlock *compile(Module *, Function *, BasicBlock *) = 0;

    virtual ~BFInstruction(){};
};

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
        Value *NewCellIndex = Builder.CreateAdd(CellIndex, IncrementAmount);

        Builder.CreateStore(NewCellIndex, CellIndexPtr);

        return BB;
    }
};

class BFLoop : public BFInstruction {
  private:
    std::vector<BFInstruction *> LoopBody;

  public:
    BFLoop(std::vector<BFInstruction *> LoopBody_) { LoopBody = LoopBody_; }

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

using BFProgram = std::vector<BFInstruction *>;

Module *compileProgram(BFProgram *Program) {
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

std::string readSource(std::string programPath) {
    std::ifstream stream(programPath);
    std::string source((std::istreambuf_iterator<char>(stream)),
                       std::istreambuf_iterator<char>());

    return source;
}

BFProgram parseSource(std::string Source) {
    // TODO: [ and ].
    BFProgram Program;

    for (auto I = Source.begin(), E = Source.end(); I != E; ++I) {
        switch (*I) {
        case '+': {
            Program.push_back(new BFIncrement(1));
            break;
        }
        case '-': {
            Program.push_back(new BFIncrement(-1));
            break;
        }
        case '>': {
            Program.push_back(new BFDataIncrement(1));
            break;
        }
        case '<': {
            Program.push_back(new BFDataIncrement(-1));
            break;
        }
        case ',': {
            Program.push_back(new BFRead);
            break;
        }
        case '.': {
            Program.push_back(new BFWrite);
            break;
        }
        default:
            // skip comments
            break;
        }
    }

    return Program;
}

void printUsage(std::string ProgramName) {
    errs() << "Usage: " << ProgramName << " <my-program.bf> \n";
}

std::string getOutputName(std::string ProgramName) {
    // Strip the path, so "../foo/bar/baz.bf" -> "baz.bf".
    std::regex FilenamePattern("[^/]+$");
    std::smatch RegexMatch;
    std::regex_search(ProgramName, RegexMatch, FilenamePattern);
    std::string FileName = RegexMatch[0].str();

    // Strip the extension "baz.bf" -> "baz"
    std::regex ExtensionPattern("\\.bf?$");
    std::string Name = std::regex_replace(FileName, ExtensionPattern, "");

    return Name + ".ll";
}

int main(int argc, char *argv[]) {
    if (argc != 2) {
        printUsage(argv[0]);
        return EXIT_FAILURE;
    }

    auto ProgramPath = std::string(argv[1]);

    if (!boost::filesystem::exists(ProgramPath)) {
        errs() << "No such file: " << ProgramPath << "\n";
        return EXIT_FAILURE;
    }

    auto Source = readSource(ProgramPath);
    auto Program = parseSource(Source);

    Module *Mod = compileProgram(&Program);

    // TODO: use a proper pointer container instead of raw pointers.
    for (auto I = Program.begin(), E = Program.end(); I != E; ++I) {
        delete *I;
    }

    // Write the LLVM IR to a file.
    std::ofstream StdOutputFile(getOutputName(ProgramPath));
    raw_os_ostream OutputFile(StdOutputFile);

    // Print the generated code
    Mod->print(OutputFile, nullptr);

    delete Mod;

    return 0;
}

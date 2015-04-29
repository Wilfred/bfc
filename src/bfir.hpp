#ifndef BFIR_HEADER
#define BFIR_HEADER

#include "llvm/IR/Verifier.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

using namespace llvm;

class BFInstruction {
  public:
    virtual std::ostream &stream_write(std::ostream &) const = 0;
    // Append the appropriate instructions to the given basic
    // block. We may also create new basic blocks, return the next
    // basic block we should append to.
    virtual BasicBlock *compile(Module &, Function &, BasicBlock &) = 0;

    virtual ~BFInstruction(){};
};

std::ostream &operator<<(std::ostream &, const BFInstruction &);
bool operator==(const BFInstruction &X, const BFInstruction &Y);
bool operator!=(const BFInstruction &X, const BFInstruction &Y);

using BFInstPtr = std::shared_ptr<BFInstruction>;

// Just like a normal vector, except we've overridden equality.
class BFProgram {
  public:
    std::vector<BFInstPtr> Instructions;

    std::ostream &stream_write(std::ostream &) const;
    void push_back(BFInstPtr);
    std::vector<BFInstPtr>::iterator begin();
    std::vector<BFInstPtr>::const_iterator begin() const;
    std::vector<BFInstPtr>::iterator end();
    std::vector<BFInstPtr>::const_iterator end() const;
    std::vector<BFInstPtr>::size_type size() const;
};

std::ostream &operator<<(std::ostream &, const BFProgram &);

bool operator==(const BFProgram &, const BFProgram &);

bool operator!=(const BFProgram &, const BFProgram &);

class BFIncrement : public BFInstruction {
  public:
    std::ostream &stream_write(std::ostream &) const;
    // TODO: can this be private?
    int Amount;

    BFIncrement();
    BFIncrement(int);

    virtual BasicBlock *compile(Module &, Function &, BasicBlock &);
};

std::ostream &operator<<(std::ostream &, const BFIncrement &);

class BFDataIncrement : public BFInstruction {
  public:
    int Amount;
    std::ostream &stream_write(std::ostream &) const;
    BFDataIncrement();
    BFDataIncrement(int);

    virtual BasicBlock *compile(Module &, Function &, BasicBlock &BB);
};

std::ostream &operator<<(std::ostream &, const BFDataIncrement &);

class BFRead : public BFInstruction {
  public:
    std::ostream &stream_write(std::ostream &) const;
    virtual BasicBlock *compile(Module &Mod, Function &, BasicBlock &BB);
};

std::ostream &operator<<(std::ostream &, const BFRead &);

class BFWrite : public BFInstruction {
  public:
    std::ostream &stream_write(std::ostream &) const;
    virtual BasicBlock *compile(Module &Mod, Function &, BasicBlock &BB);
};

std::ostream &operator<<(std::ostream &, const BFWrite &);

class BFLoop : public BFInstruction {
  public:
    BFProgram LoopBody;
    std::ostream &stream_write(std::ostream &) const;
    BFLoop(BFProgram);

    virtual BasicBlock *compile(Module &Mod, Function &F, BasicBlock &BB);
};

std::ostream &operator<<(std::ostream &, const BFLoop &);

Module *compileProgram(BFProgram &);

BFProgram parseSource(std::string &);

BFProgram coalesceIncrements(BFProgram &);

#endif

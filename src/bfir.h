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
    // Append the appropriate instructions to the given basic
    // block. We may also create new basic blocks, return the next
    // basic block we should append to.
    virtual BasicBlock *compile(Module *, Function *, BasicBlock *) = 0;

    virtual ~BFInstruction(){};
};

bool operator==(const BFInstruction &X, const BFInstruction &Y);
bool operator!=(const BFInstruction &X, const BFInstruction &Y);

using BFInstPtr = std::shared_ptr<BFInstruction>;

// Just like a normal vector, except we've overridden equality.
class BFSequence {
    std::vector<BFInstPtr> Instructions;

  public:
    void push_back(BFInstPtr);
    std::vector<BFInstPtr>::iterator begin();
    std::vector<BFInstPtr>::iterator end();
    std::vector<BFInstPtr>::size_type size() const;
};

bool operator==(const BFSequence &, const BFSequence &);

bool operator!=(const BFSequence &, const BFSequence &);

class BFIncrement : public BFInstruction {
  public:
    int Amount;

    BFIncrement();
    BFIncrement(int);

    virtual BasicBlock *compile(Module *, Function *, BasicBlock *);
};

class BFDataIncrement : public BFInstruction {
  private:
    int Amount;

  public:
    BFDataIncrement();
    BFDataIncrement(int);

    virtual BasicBlock *compile(Module *, Function *, BasicBlock *BB);
};

class BFRead : public BFInstruction {
  public:
    virtual BasicBlock *compile(Module *Mod, Function *, BasicBlock *BB);
};

class BFWrite : public BFInstruction {
  public:
    virtual BasicBlock *compile(Module *Mod, Function *, BasicBlock *BB);
};

class BFLoop : public BFInstruction {
  private:
    BFSequence LoopBody;

  public:
    BFLoop(BFSequence);

    virtual BasicBlock *compile(Module *Mod, Function *F, BasicBlock *BB);
};

BFSequence parseSource(std::string);

Module *compileProgram(BFSequence *);

#endif

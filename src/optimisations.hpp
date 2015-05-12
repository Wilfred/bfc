#ifndef OPTIMISATIONS_HEADER
#define OPTIMISATIONS_HEADER

#include "bfir.hpp"

BFProgram markKnownZero(const BFProgram &);

BFProgram combineIncrements(BFProgram &);

BFProgram combineDataIncrements(BFProgram &);

BFProgram combineSetAndIncrements(BFProgram &);

#endif

#ifndef OPTIMISATIONS_HEADER
#define OPTIMISATIONS_HEADER

#include "bfir.hpp"

BFProgram markKnownZero(const BFProgram &);

BFProgram coalesceIncrements(BFProgram &);

BFProgram coalesceDataIncrements(BFProgram &);

#endif

#ifndef OPTIMISATIONS_HEADER
#define OPTIMISATIONS_HEADER

#include "bfir.hpp"

BFProgram markKnownZero(const BFProgram &);

BFProgram combineIncrements(const BFProgram &);

BFProgram combineDataIncrements(const BFProgram &);

BFProgram combineSetAndIncrements(const BFProgram &);

BFProgram combineSets(const BFProgram &);

BFProgram simplifyZeroingLoop(const BFProgram &);

BFProgram applyAllPasses(const BFProgram &);

#endif

#include "bfir.hpp"

BFProgram markKnownZero(const BFProgram &Sequence) {
    BFProgram Result = Sequence;

    // At the start of execution, cell #0 is 0.
    BFInstPtr Ptr(new BFSet(0));
    Result.insert(Result.begin(), Ptr);

    return Result;
}

// TODO: combine instructions inside loop bodies too.
BFProgram combineIncrements(const BFProgram &Sequence) {
    BFProgram Result;

    // TODO: use an option data type instead of a pointer to a pointer
    // just for nullability.
    BFInstPtr *Last = nullptr;

    for (const BFInstPtr &Current : Sequence) {
        if (Last == nullptr) {
            Last = (BFInstPtr *)&Current;
        } else {
            try {
                BFIncrement &LastIncr = dynamic_cast<BFIncrement &>(**Last);
                BFIncrement &CurrentIncr =
                    dynamic_cast<BFIncrement &>(*Current);

                int Sum = CurrentIncr.Amount + LastIncr.Amount;
                // TODO: we should wrap-around amounts at our maximum cell
                // value.
                if (Sum == 0) {
                    Last = nullptr;
                } else {
                    Last = new BFInstPtr(new BFIncrement(Sum));
                }

            } catch (const std::bad_cast &) {
                Result.push_back(*Last);
                Last = (BFInstPtr *)&Current;
            }
        }
    }

    if (Last != nullptr) {
        Result.push_back(*Last);
    }

    return Result;
}

BFProgram combineDataIncrements(const BFProgram &Sequence) {
    BFProgram Result;

    // TODO: use an option data type instead of a pointer to a pointer
    // just for nullability.
    BFInstPtr *Last = nullptr;

    for (const BFInstPtr &Current : Sequence) {
        if (Last == nullptr) {
            Last = (BFInstPtr *)&Current;
        } else {
            try {
                BFDataIncrement &LastIncr =
                    dynamic_cast<BFDataIncrement &>(**Last);
                BFDataIncrement &CurrentIncr =
                    dynamic_cast<BFDataIncrement &>(*Current);

                int Sum = CurrentIncr.Amount + LastIncr.Amount;
                if (Sum == 0) {
                    Last = nullptr;
                } else {
                    Last = new BFInstPtr(new BFDataIncrement(Sum));
                }

            } catch (const std::bad_cast &) {
                Result.push_back(*Last);
                Last = (BFInstPtr *)&Current;
            }
        }
    }

    if (Last != nullptr) {
        Result.push_back(*Last);
    }

    return Result;
}

BFProgram combineSetAndIncrements(const BFProgram &Sequence) {
    BFProgram Result;

    BFInstPtr *Last = nullptr;

    for (const BFInstPtr &Current : Sequence) {
        if (Last == nullptr) {
            Last = (BFInstPtr *)&Current;
        } else {
            try {
                BFSet &LastSet = dynamic_cast<BFSet &>(**Last);
                BFIncrement &CurrentIncr =
                    dynamic_cast<BFIncrement &>(*Current);

                int Sum = LastSet.Amount + CurrentIncr.Amount;
                Last = new BFInstPtr(new BFSet(Sum));

            } catch (const std::bad_cast &) {
                Result.push_back(*Last);
                Last = (BFInstPtr *)&Current;
            }
        }
    }

    if (Last != nullptr) {
        Result.push_back(*Last);
    }

    return Result;
}

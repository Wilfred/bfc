#include "bfir.hpp"

BFProgram markKnownZero(const BFProgram &Sequence) {
    BFProgram Result = Sequence;

    // At the start of execution, cell #0 is 0.
    BFInstPtr Ptr(new BFSet(0));
    Result.insert(Result.begin(), Ptr);

    return Result;
}

// TODO: run coalesce inside loop bodies too.
BFProgram coalesceIncrements(BFProgram &Sequence) {
    BFProgram Result;

    // TODO: use an option data type instead of a pointer to a pointer
    // just for nullability.
    BFInstPtr *Last = nullptr;

    for (BFInstPtr &Current : Sequence) {
        if (Last == nullptr) {
            Last = &Current;
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
                Last = &Current;
            }
        }
    }

    if (Last != nullptr) {
        Result.push_back(*Last);
    }

    return Result;
}

BFProgram coalesceDataIncrements(BFProgram &Sequence) {
    BFProgram Result;

    // TODO: use an option data type instead of a pointer to a pointer
    // just for nullability.
    BFInstPtr *Last = nullptr;

    for (BFInstPtr &Current : Sequence) {
        if (Last == nullptr) {
            Last = &Current;
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
                Last = &Current;
            }
        }
    }

    if (Last != nullptr) {
        Result.push_back(*Last);
    }

    return Result;
}

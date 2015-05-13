#include <fstream>

#include "llvm/Support/raw_ostream.h"
#include "bfir.hpp"

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
            // TODO: use std::make_shared instead.
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

std::string readSource(std::string programPath) {
    std::ifstream stream(programPath);
    std::string source((std::istreambuf_iterator<char>(stream)),
                       std::istreambuf_iterator<char>());

    return source;
}

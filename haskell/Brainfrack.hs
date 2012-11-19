module Brainfrack (evalProgram) where

import Data.Word (Word8)
import Data.Char (ord, chr)
import Control.Monad (liftM)

type Program = String

replaceInList [] y _ = error "Index is too big for list"
replaceInList (x:xs) y 0 = y:xs
replaceInList (x:xs) y index = x:replaceInList xs y (index - 1)



findMatchingBracket :: Program -> Int -> Int
findMatchingBracket program index = findClosingBracket program index 0

-- TODO: only covers opening brackets
findClosingBracket :: Program -> Int -> Int -> Int
findClosingBracket program index depth
  | depth == 0 = index
  | otherwise =
    case instruction of
      '[' -> findClosingBracket program (index + 1) (depth + 1)
      ']' -> findClosingBracket program (index + 1) (depth - 1)
      _   -> findClosingBracket program (index + 1) depth
  where
    instruction = program !! index

-- this could be faster using an array instead of a list of cells
-- todo: use word8 instead of Int for cells
-- todo: we need to terminate at the end of the program
evalProgram' :: Program -> Int -> Int -> [Int] -> IO ()
evalProgram' program instructionPointer cellPointer cells =
  case instruction of
    '>' -> evalProgram' program (instructionPointer+1) (cellPointer+1) cells
    '<' -> evalProgram' program (instructionPointer+1) (cellPointer-1) cells
    '+' ->
      evalProgram' program (instructionPointer+1) cellPointer cells'
      where
        updatedCell = (cells !! cellPointer) + 1
        cells' = replaceInList cells updatedCell cellPointer
    '-' ->
      evalProgram' program (instructionPointer+1) cellPointer cells'
      where
        updatedCell = (cells !! cellPointer) - 1
        cells' = replaceInList cells updatedCell cellPointer
    '.' -> do
      let charToPrint = chr (cells !! cellPointer)
      putStr [charToPrint]
      evalProgram' program (instructionPointer+1) cellPointer cells
    ',' -> do
      updatedCell <- liftM ord getChar
      let cells' = replaceInList cells updatedCell cellPointer
      evalProgram' program (instructionPointer+1) cellPointer cells
    '[' -> undefined
    ']' -> undefined
    _ -> return ()
  where
    instruction = program !! instructionPointer


-- evalProgram :: String -> IO ()

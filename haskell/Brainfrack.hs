import Data.Word (Word8)
import Data.Char (ord, chr)
import Control.Monad (liftM)

replaceInList [] y _ = error "Index is too big for list"
replaceInList (x:xs) y 0 = y:xs
replaceInList (x:xs) y index = x:replaceInList xs y (index - 1)

-- this could be faster using an array instead of a list of cells
-- todo: use word8 instead of Int for cells
-- todo: we need to terminate at the end of the program
evalProgram' :: String -> Int -> Int -> [Int] -> IO ()
evalProgram' program programPointer dataPointer cells =
  case instruction of
    '>' -> evalProgram' program (programPointer+1) (dataPointer+1) cells
    '<' -> evalProgram' program (programPointer+1) (dataPointer-1) cells
    '+' ->
      evalProgram' program (programPointer+1) dataPointer cells'
      where
        updatedCell = (cells !! dataPointer) + 1
        cells' = replaceInList cells updatedCell dataPointer
    '-' ->
      evalProgram' program (programPointer+1) dataPointer cells'
      where
        updatedCell = (cells !! dataPointer) - 1
        cells' = replaceInList cells updatedCell dataPointer
    '.' -> do
      let charToPrint = chr (cells !! dataPointer)
      putStr [charToPrint]
      evalProgram' program (programPointer+1) dataPointer cells
    ',' -> do
      updatedCell <- liftM ord getChar
      let cells' = replaceInList cells updatedCell dataPointer
      evalProgram' program (programPointer+1) dataPointer cells
    '[' -> undefined
    ']' -> undefined
    _ -> return ()
  where
    instruction = program !! programPointer


-- evalProgram :: String -> IO ()

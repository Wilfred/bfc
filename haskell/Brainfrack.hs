import Data.Char (ord, chr)
import Control.Monad (liftM)

type Program = String

replaceInList :: Eq a => [a] -> a -> Int -> [a]
replaceInList [] _ _ = error "Index is too big for list"
replaceInList (_:xs) y 0 = y:xs
replaceInList (x:xs) y index = x:replaceInList xs y (index - 1)


type BracketPosition = Int


findClosingBracket :: Program -> BracketPosition -> BracketPosition
findClosingBracket program openIndex = findClosingBracket' (openIndex+1) (1 :: Integer)
  where
    -- iterate through the program and find the first unbalanced closing bracket
    findClosingBracket' index depth
      | depth == 0 = index - 1
      | otherwise =
        case program !! index of
          '[' -> findClosingBracket' (index + 1) (depth + 1)
          ']' -> findClosingBracket' (index + 1) (depth - 1)
          _   -> findClosingBracket' (index + 1) depth
          
findOpeningBracket :: Program -> BracketPosition -> BracketPosition
findOpeningBracket program closeIndex = findOpeningBracket' 0
  where
    -- iterate through the program and find the opening bracket which matches this closing bracket
    findOpeningBracket' index =
      case program !! index of
        '[' -> if closeIndex == (findClosingBracket program index) then 
                 index
               else
                 findOpeningBracket' (index + 1)
        _ -> findOpeningBracket' (index + 1)


type Cells = [Int]


-- this could be faster using an array instead of a list of cells
-- todo: use word8 instead of Int for cells
evalProgram' :: Program -> Int -> Int -> Cells -> IO ()
evalProgram' program instructionIndex cellIndex cells
  | instructionIndex >= length program = return ()
  | otherwise =
    case program !! instructionIndex of
      '>' -> evalProgram' program (instructionIndex+1) (cellIndex+1) cells
      '<' -> evalProgram' program (instructionIndex+1) (cellIndex-1) cells
      '+' ->
        evalProgram' program (instructionIndex+1) cellIndex cells'
        where
          updatedCell = (cells !! cellIndex) + 1
          cells' = replaceInList cells updatedCell cellIndex
      '-' ->
        evalProgram' program (instructionIndex+1) cellIndex cells'
        where
          updatedCell = (cells !! cellIndex) - 1
          cells' = replaceInList cells updatedCell cellIndex
      '.' -> do
        let charToPrint = chr (cells !! cellIndex)
        putStr [charToPrint]
        evalProgram' program (instructionIndex+1) cellIndex cells
      ',' -> do
        updatedCell <- liftM ord getChar
        let cells' = replaceInList cells updatedCell cellIndex
        evalProgram' program (instructionIndex+1) cellIndex cells'
      '[' -> do
        case cells !! cellIndex of
          0 -> evalProgram' program (closingIndex+1) cellIndex cells
            where 
              closingIndex = findClosingBracket program instructionIndex
          _ -> evalProgram' program (instructionIndex+1) cellIndex cells
      ']' -> do
        let openingIndex = findOpeningBracket program instructionIndex
        evalProgram' program openingIndex cellIndex cells
      _ -> evalProgram' program (instructionIndex+1) cellIndex cells


evalProgram :: String -> IO ()
evalProgram program = evalProgram' program 0 0 initialCells
  where
    initialCells = [0 | _ <- [1 .. 30000 :: Integer]]

main :: IO ()
main = do
  program <- getContents
  evalProgram program

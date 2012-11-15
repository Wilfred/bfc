import Data.Word (Word8)

replaceInList [] y _ = error "Index is too big for list"
replaceInList (x:xs) y 0 = y:xs
replaceInList (x:xs) y index = x:replaceInList xs y (index - 1)


evalProgram' :: String -> Int -> Int -> [Word8] -> IO ()
evalProgram' (instruction:program) programPointer dataPointer cells =
  case instruction of
    '>' -> evalProgram' (instruction:program) programPointer (dataPointer+1) cells
    '<' -> evalProgram' (instruction:program) programPointer (dataPointer-1) cells
    '+' -> undefined
    '-' -> undefined
    '.' -> undefined
    ',' -> undefined
    '[' -> undefined
    ']' -> undefined

-- evalProgram :: String -> IO ()

declare noalias i8* @calloc(i32, i32)
declare i32 @putchar(i32)
declare i32 @getchar()

; run with
; $ echo a | lli comma.ll
define i32 @main() nounwind {
       %cells = call i8* @calloc(i32 3000, i32 1)
       %cell_index = alloca i8
       store i8 0, i8* %cell_index

       %cell_index_val = load i8* %cell_index

       ; we implement the BF program ',.'
       %cell_ptr = getelementptr i8* %cells, i8 %cell_index_val

       ; read a character from stdin and save it in the cell
       %input_int = call i32 @getchar()
       %input_byte = trunc i32 %input_int to i8
       store i8 %input_byte, i8* %cell_ptr

       ; print the current cell
       %tmp = load i8* %cell_ptr
       %tmp2 = sext i8 %tmp to i32
       %1 = call i32 @putchar(i32 %tmp2)

       ret i32 0
}

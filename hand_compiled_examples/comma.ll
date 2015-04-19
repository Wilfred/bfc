declare i8* @calloc(i32)
declare void @free(i8*)
declare i32 @putchar(i32)
declare i32 @getchar()

; run with
; $ echo a | lli comma.ll
define i32 @main() nounwind {
       %cells = call i8* @calloc(i32 30000)
       %cell_index_ptr = alloca i32
       store i32 0, i32* %cell_index_ptr

       ; we implement the BF program ',.'
       
       %cell_index = load i32* %cell_index_ptr
       %cell_ptr = getelementptr i8* %cells, i32 %cell_index

       ; read a character from stdin and save it in the cell
       %input_int = call i32 @getchar()
       %input_byte = trunc i32 %input_int to i8
       store i8 %input_byte, i8* %cell_ptr

       ; print the current cell
       %current_cell = load i8* %cell_ptr
       %current_cell_word = sext i8 %current_cell to i32
       call i32 @putchar(i32 %current_cell_word)

       ret i32 0
}

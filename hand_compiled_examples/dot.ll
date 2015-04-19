declare i8* @calloc(i32)
declare void @free(i8*)
declare i32 @putchar(i32)
declare i32 @getchar()

define i32 @main() nounwind {
       %cells = call i8* @calloc(i32 30000)
       %cell_index_ptr = alloca i32
       store i32 0, i32* %cell_index_ptr

       ; we implement the BF program '+++++ ++++ .'
       ; so we print '\t' to stdout.

       %cell_index = load i32* %cell_index_ptr
       %cell_ptr = getelementptr i8* %cells, i32 %cell_index

       ; increment 9 times.
       %tmp = load i8* %cell_ptr
       %tmp2 = add i8 %tmp, 1
       store i8 %tmp2, i8* %cell_ptr

       %tmp3 = load i8* %cell_ptr
       %tmp4 = add i8 %tmp3, 1
       store i8 %tmp4, i8* %cell_ptr

       %tmp5 = load i8* %cell_ptr
       %tmp6 = add i8 %tmp5, 1
       store i8 %tmp6, i8* %cell_ptr

       %tmp7 = load i8* %cell_ptr
       %tmp8 = add i8 %tmp7, 1
       store i8 %tmp8, i8* %cell_ptr

       %tmp9 = load i8* %cell_ptr
       %tmp10 = add i8 %tmp9, 1
       store i8 %tmp10, i8* %cell_ptr

       %tmp11 = load i8* %cell_ptr
       %tmp12 = add i8 %tmp11, 1
       store i8 %tmp12, i8* %cell_ptr

       %tmp13 = load i8* %cell_ptr
       %tmp14 = add i8 %tmp13, 1
       store i8 %tmp14, i8* %cell_ptr

       %tmp15 = load i8* %cell_ptr
       %tmp16 = add i8 %tmp15, 1
       store i8 %tmp16, i8* %cell_ptr

       %tmp17 = load i8* %cell_ptr
       %tmp18 = add i8 %tmp17, 1
       store i8 %tmp18, i8* %cell_ptr

       ; print the current cell
       %current_cell = load i8* %cell_ptr
       %current_cell_word = sext i8 %current_cell to i32
       call i32 @putchar(i32 %current_cell_word)

       ret i32 0
}

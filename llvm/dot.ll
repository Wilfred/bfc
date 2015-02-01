declare noalias i8* @calloc(i64)
declare i32 @putchar(i32)

define i32 @main() nounwind {
       %cells = call i8* @calloc(i64 3000)
       %cell_index = alloca i8

       %cell_index_val = load i8* %cell_index

       ; we implement the BF program '+++++ ++++ .'
       ; so we print '\t' to stdout.
       %cell_ptr = getelementptr i8* %cells, i8 %cell_index_val

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

       ; print the current value
       %tmp19 = load i8* %cell_ptr
       %tmp20 = sext i8 %tmp19 to i32
       %1 = call i32 @putchar(i32 %tmp20)

       ret i32 0
}

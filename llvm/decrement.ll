declare noalias i8* @malloc(i64)

define i32 @main() nounwind {
       %cells = call i8* @malloc(i64 3000)
       %cell_index = alloca i8
       %data_index = alloca i8

       %cell_index_val = load i8* %cell_index

       ; we implement the BF program '-'
       %cell_ptr = getelementptr i8* %cells, i8 %cell_index_val
       %tmp = load i8* %cell_ptr
       %tmp2 = add i8 1, %tmp
       store i8 %tmp2, i8* %cell_ptr

       ret i32 0
}

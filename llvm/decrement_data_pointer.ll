declare noalias i8* @calloc(i64)

define i32 @main() nounwind {
       %cells = call i8* @calloc(i64 3000)
       %cell_index = alloca i8

       ; we implement the BF program '><+'

       ; increment the cell_index
       %cell_index_tmp = load i8* %cell_index
       %cell_index_tmp2 = add i8 1, %cell_index_tmp
       store i8 %cell_index_tmp2, i8* %cell_index

       ; decrement the cell_index
       %cell_index_tmp3 = load i8* %cell_index
       %cell_index_tmp4 = sub i8 %cell_index_tmp3, 1
       store i8 %cell_index_tmp4, i8* %cell_index

       ; increment this cell
       %cell_index_tmp5 = load i8* %cell_index
       %cell_ptr2 = getelementptr i8* %cells, i8 %cell_index_tmp5
       %cell_value_tmp = load i8* %cell_ptr2
       %cell_value_tmp2 = add i8 %cell_value_tmp, 1
       store i8 %cell_value_tmp2, i8* %cell_ptr2

       ; exit the stored value, as a sanity check
       %exit_code_byte = load i8* %cell_ptr2
       %exit_code = zext i8 %exit_code_byte to i32
       ret i32 %exit_code
}

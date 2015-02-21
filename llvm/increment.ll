declare i8* @calloc(i32)
declare void @free(i8*)

define i32 @main() nounwind {
       %cells = call i8* @calloc(i32 30000)
       %cell_index_ptr = alloca i32
       store i32 0, i32* %cell_index_ptr

       ; we implement the BF program '+'

       %cell_index = load i32* %cell_index_ptr
       %cell_ptr = getelementptr i8* %cells, i32 %cell_index
       %cell_value = load i8* %cell_ptr
       %new_cell_value = add i8 %cell_value, 1
       store i8 %new_cell_value, i8* %cell_ptr

       ; exit the stored value, as a sanity check
       %exit_code_byte = load i8* %cell_ptr
       %exit_code = zext i8 %exit_code_byte to i32

       call void @free(i8* %cells)

       ret i32 %exit_code
}

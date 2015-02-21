declare i8* @calloc(i32)
declare void @free(i8*)

define i32 @main() {
       %cells = call i8* @calloc(i32 30000)
       %cell_index_ptr = alloca i32
       store i32 0, i32* %cell_index_ptr

       ; We implement the BF program '<'.
       ; Note strictly speaking, a negative cell index is undefined in BF.

       ; decrement the cell_index
       %cell_index = load i32* %cell_index_ptr
       %new_cell_index = sub i32 %cell_index, 1
       store i32 %new_cell_index, i32* %cell_index_ptr

       call void @free(i8* %cells)

       ; sanity check: exit((int)cell_index);
       %exit_code = load i32* %cell_index_ptr
       ret i32 %exit_code
}

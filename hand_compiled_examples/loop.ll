declare noalias i8* @calloc(i32, i32)

define i32 @main() nounwind {
       %cells = call i8* @calloc(i32 3000, i32 1)
       %cell_index = alloca i8
       store i8 0, i8* %cell_index

       br label %loop

       ; we implement the BF program '[-]'
loop:
       %cell_index_val = load i8* %cell_index
       %cell_ptr = getelementptr i8* %cells, i8 %cell_index_val

       ; see if we should continue looping
       %cmp_value = load i8* %cell_ptr
       %is_zero = icmp eq i8 %cmp_value, 0
       br i1 %is_zero, label %end, label %decrement

decrement:
       %tmp = load i8* %cell_ptr
       %tmp2 = add i8 %tmp, 1
       store i8 %tmp2, i8* %cell_ptr

       br label %loop

end:
       ; exit the stored value, as a sanity check
       %exit_code_byte = load i8* %cell_ptr
       %exit_code = zext i8 %exit_code_byte to i32
       ret i32 %exit_code
}

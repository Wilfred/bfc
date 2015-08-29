use std::collections::HashMap;
use std::ffi::CString;
use std::num::Wrapping;

use llvm::compile_to_ir;
use bfir::Instruction::*;

#[test]
fn compile_loop() {
    let result = compile_to_ir("foo", &vec![Loop(vec![Increment(Wrapping(1))])],
                               &vec![0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %loop_header

loop_header:                                      ; preds = %loop_body, %entry
  %cell_index = load i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8* %cells, i32 %cell_index
  %cell_value = load i8* %current_cell_ptr
  %cell_value_is_zero = icmp eq i8 0, %cell_value
  br i1 %cell_value_is_zero, label %loop_after, label %loop_body

loop_body:                                        ; preds = %loop_header
  %cell_index1 = load i32* %cell_index_ptr
  %current_cell_ptr2 = getelementptr i8* %cells, i32 %cell_index1
  %cell_value3 = load i8* %current_cell_ptr2
  %new_cell_value = add i8 %cell_value3, 1
  store i8 %new_cell_value, i8* %current_cell_ptr2
  br label %loop_header

loop_after:                                       ; preds = %loop_header
  ret i32 0
}

attributes #0 = { nounwind }
";
    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_empty_program() {
    let result = compile_to_ir("foo", &vec![], &vec![0; 10], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  ret i32 0
}

attributes #0 = { nounwind }
";
    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_set() {
    let result = compile_to_ir("foo", &vec![Set(Wrapping(1))], &vec![0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  %cell_index = load i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8* %cells, i32 %cell_index
  store i8 1, i8* %current_cell_ptr
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn respect_initial_cell_ptr() {
    let result = compile_to_ir("foo", &vec![PointerIncrement(1)], &vec![0; 10], 8, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8, i32 10
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 10, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 8, i32* %cell_index_ptr
  %cell_index = load i32* %cell_index_ptr
  %new_cell_index = add i32 %cell_index, 1
  store i32 %new_cell_index, i32* %cell_index_ptr
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_multiply_move() {
    let mut changes = HashMap::new();
    changes.insert(1, Wrapping(2));
    changes.insert(2, Wrapping(3));
    let result = compile_to_ir("foo", &vec![MultiplyMove(changes)], &vec![0, 0, 0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8, i32 3
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  %cell_index = load i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8* %cells, i32 %cell_index
  %cell_value = load i8* %current_cell_ptr
  store i8 0, i8* %current_cell_ptr
  %target_cell_ptr = getelementptr i8* %current_cell_ptr, i32 1
  %target_cell_val = load i8* %target_cell_ptr
  %additional_val = mul i8 %cell_value, 2
  %new_target_val = add i8 %target_cell_val, %additional_val
  store i8 %new_target_val, i8* %target_cell_ptr
  %target_cell_ptr1 = getelementptr i8* %current_cell_ptr, i32 2
  %target_cell_val2 = load i8* %target_cell_ptr1
  %additional_val3 = mul i8 %cell_value, 3
  %new_target_val4 = add i8 %target_cell_val2, %additional_val3
  store i8 %new_target_val4, i8* %target_cell_ptr1
  ret i32 0
}

attributes #0 = { nounwind }
";

    println!("{}", String::from_utf8_lossy(result.as_bytes()));

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn set_initial_cell_values() {
    let result = compile_to_ir("foo", &vec![PointerIncrement(1)], &vec![1, 1, 2, 0, 0, 0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8, i32 6
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 1, i32 2, i32 1, i1 true)
  %offset_cell_ptr1 = getelementptr i8* %cells, i32 2
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr1, i8 2, i32 1, i32 1, i1 true)
  %offset_cell_ptr2 = getelementptr i8* %cells, i32 3
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr2, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  %cell_index = load i32* %cell_index_ptr
  %new_cell_index = add i32 %cell_index, 1
  store i32 %new_cell_index, i32* %cell_index_ptr
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_static_outputs() {
    let result = compile_to_ir("foo", &vec![], &vec![], 0, &vec![5, 10]);
    let expected = "; ModuleID = \'foo\'

@known_outputs = constant [2 x i8] c\"\\05\\0A\"

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %0 = call i32 @write(i32 1, i8* getelementptr inbounds ([2 x i8]* @known_outputs, i32 0, i32 0), i32 2)
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_ptr_increment() {
    let result = compile_to_ir("foo", &vec![PointerIncrement(1)], &vec![0, 0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8, i32 2
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 2, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  %cell_index = load i32* %cell_index_ptr
  %new_cell_index = add i32 %cell_index, 1
  store i32 %new_cell_index, i32* %cell_index_ptr
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_increment() {
    let result = compile_to_ir("foo", &vec![Increment(Wrapping(1))], &vec![0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = alloca i8
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  %cell_index = load i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8* %cells, i32 %cell_index
  %cell_value = load i8* %current_cell_ptr
  %new_cell_value = add i8 %cell_value, 1
  store i8 %new_cell_value, i8* %current_cell_ptr
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

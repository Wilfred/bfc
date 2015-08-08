use llvm::compile_to_ir;
use bfir::Instruction::*;
use std::ffi::CString;

#[test]
fn compile_loop() {
    let result = compile_to_ir("foo", &vec![Loop(vec![Increment(1)])], &vec![0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

declare i8* @malloc(i32)

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare void @free(i8*)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = call i8* @malloc(i32 1)
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
  call void @free(i8* %cells)
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

declare i8* @malloc(i32)

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare void @free(i8*)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = call i8* @malloc(i32 10)
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 10, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { nounwind }
";
    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn respect_initial_cell_ptr() {
    // TODO: this is a bad test, we would never access cell 42 with 10
    // cells.
    let result = compile_to_ir("foo", &vec![], &vec![0; 10], 42, &vec![]);
    let expected = "; ModuleID = \'foo\'

declare i8* @malloc(i32)

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare void @free(i8*)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = call i8* @malloc(i32 10)
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 10, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 42, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { nounwind }
";
    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn set_initial_cell_values() {
    let result = compile_to_ir("foo", &vec![], &vec![1, 1, 2, 0, 0, 0], 0, &vec![]);
    let expected = "; ModuleID = \'foo\'

declare i8* @malloc(i32)

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare void @free(i8*)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = call i8* @malloc(i32 6)
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 1, i32 2, i32 1, i1 true)
  %offset_cell_ptr1 = getelementptr i8* %cells, i32 2
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr1, i8 2, i32 1, i32 1, i1 true)
  %offset_cell_ptr2 = getelementptr i8* %cells, i32 3
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr2, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

#[test]
fn compile_static_outputs() {
    let result = compile_to_ir("foo", &vec![], &vec![0; 3], 0, &vec![5, 10]);
    let expected = "; ModuleID = \'foo\'

declare i8* @malloc(i32)

; Function Attrs: nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture, i8, i32, i32, i1) #0

declare void @free(i8*)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = call i8* @malloc(i32 3)
  %offset_cell_ptr = getelementptr i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  %0 = call i32 @putchar(i32 5)
  %1 = call i32 @putchar(i32 10)
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}


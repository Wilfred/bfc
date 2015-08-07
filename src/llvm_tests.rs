use llvm::compile_to_ir;
use std::ffi::CString;

#[test]
fn compile_empty_program() {
    let result = compile_to_ir("foo", &vec![], &vec![0; 10], 0);
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
  call void @llvm.memset.p0i8.i32(i8* %cells, i8 0, i32 10, i32 1, i1 true)
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
    let result = compile_to_ir("foo", &vec![], &vec![0; 10], 42);
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
  call void @llvm.memset.p0i8.i32(i8* %cells, i8 0, i32 10, i32 1, i1 true)
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
    let result = compile_to_ir("foo", &vec![], &vec![1, 1, 2, 0, 0, 0], 0);
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
  call void @llvm.memset.p0i8.i32(i8* %cells, i8 1, i32 2, i32 1, i1 true)
  call void @llvm.memset.p0i8.i32(i8* %cells, i8 2, i32 1, i32 1, i1 true)
  call void @llvm.memset.p0i8.i32(i8* %cells, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { nounwind }
";

    assert_eq!(result, CString::new(expected).unwrap());
}

use llvm::compile_to_ir;
use std::ffi::CString;

#[test]
fn compile_empty_program() {
    let result = compile_to_ir("foo", &vec![], 10);
    let expected = "; ModuleID = \'foo\'

declare i8* @calloc(i32, i32)

declare void @free(i8*)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
entry:
  %cells = call i8* @calloc(i32 10, i32 1)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}
";
    assert_eq!(result, CString::new(expected).unwrap());
}

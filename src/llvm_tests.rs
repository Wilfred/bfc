use llvm::compile_to_ir;
use std::ffi::CString;

#[test]
fn compile_empty_program() {
    let result = compile_to_ir("foo", &vec![], 10);
    let expected = "; ModuleID = \'foo\'\n\ndeclare i8* @calloc(i32, i32)\n\ndeclare void @free(i8*)\n\ndeclare i32 @putchar(i32)\n\ndeclare i32 @getchar()\n\ndefine i32 @main() {\nentry:\n  %cells = call i8* @calloc(i32 10, i32 1)\n  %cell_index_ptr = alloca i32\n  store i32 0, i32* %cell_index_ptr\n  call void @free(i8* %cells)\n  ret i32 0\n}\n";
    assert_eq!(result, CString::new(expected).unwrap());
}

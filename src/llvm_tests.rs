use std::collections::HashMap;
use std::ffi::CString;
use std::num::Wrapping;

use bfir::AstNode::*;
use bfir::Position;
use execution::ExecutionState;
use itertools::EitherOrBoth::Both;
use itertools::Itertools;
use llvm::compile_to_module;

/// Assert that two CString values are equal. If they're not, print
/// the strings nicely (e.g. '\n' as an actual newline).
macro_rules! assert_cstring_eq {
    ($actual:expr, $expected:expr) => {{
        let expected = $expected;
        let expected_str = expected.to_string_lossy();
        let actual = $actual;
        let actual_str = actual.to_string_lossy();

        if expected_str != actual_str {
            let expected_lines = expected_str.split('\n');
            let actual_lines = actual_str.split('\n');
            println!("Lines do not match!");

            for differing_line in
                expected_lines
                    .zip_longest(actual_lines)
                    .filter(|pair| match *pair {
                        Both(x, y) => x != y,
                        _ => true,
                    })
            {
                println!("{:?}", differing_line);
            }
            assert!(false);
        }
    }};
}

#[test]
fn compile_loop() {
    let instrs = vec![Loop {
        body: vec![Increment {
            amount: Wrapping(1),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        }],
        position: Some(Position { start: 0, end: 0 }),
    }];

    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 1)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  br label %loop_header

loop_header:                                      ; preds = %loop_body, %after_init
  %cell_index = load i32, i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %cell_index
  %cell_value = load i8, i8* %current_cell_ptr
  %cell_value_is_zero = icmp eq i8 0, %cell_value
  br i1 %cell_value_is_zero, label %loop_after, label %loop_body

loop_body:                                        ; preds = %loop_header
  %cell_index1 = load i32, i32* %cell_index_ptr
  %offset_cell_index = add i32 %cell_index1, 0
  %current_cell_ptr2 = getelementptr i8, i8* %cells, i32 %offset_cell_index
  %cell_value3 = load i8, i8* %current_cell_ptr2
  %new_cell_value = add i8 %cell_value3, 1
  store i8 %new_cell_value, i8* %current_cell_ptr2
  br label %loop_header

loop_after:                                       ; preds = %loop_header
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";
    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_empty_program() {
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &[],
        &ExecutionState {
            start_instr: None,
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  br label %beginning

beginning:                                        ; preds = %init
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";
    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_set_with_offset() {
    let instrs = vec![Set {
        amount: Wrapping(1),
        offset: 42,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); 50],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 50)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 50, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %offset_cell_index = add i32 %cell_index, 42
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %offset_cell_index
  store i8 1, i8* %current_cell_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_read() {
    let instrs = vec![Read { position: None }];

    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        },
    );

    let expected = "; ModuleID = 'foo'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 1)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %cell_index
  %input_char = call i32 @getchar()
  %input_byte = trunc i32 %input_char to i8
  store i8 %input_byte, i8* %current_cell_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    println!("actual: {}", result.to_cstring().to_str().unwrap());
    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_write() {
    let instrs = vec![Write { position: None }];

    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        },
    );

    let expected = "; ModuleID = 'foo'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 1)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %cell_index
  %cell_value = load i8, i8* %current_cell_ptr
  %cell_val_as_char = sext i8 %cell_value to i32
  %0 = call i32 @putchar(i32 %cell_val_as_char)
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn respect_initial_cell_ptr() {
    let instrs = vec![PointerIncrement {
        amount: 1,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); 10],
            cell_ptr: 8,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 10)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 10, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 8, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %new_cell_index = add i32 %cell_index, 1
  store i32 %new_cell_index, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_multiply_move() {
    let mut changes = HashMap::new();
    changes.insert(1, Wrapping(2));
    changes.insert(2, Wrapping(3));
    let instrs = vec![MultiplyMove {
        changes: changes,
        position: Some(Position { start: 0, end: 0 }),
    }];

    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); 3],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 3)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %cell_index
  %cell_value = load i8, i8* %current_cell_ptr
  %cell_value_is_zero = icmp eq i8 0, %cell_value
  br i1 %cell_value_is_zero, label %multiply_after, label %multiply_body

multiply_body:                                    ; preds = %after_init
  store i8 0, i8* %current_cell_ptr
  %target_cell_ptr = getelementptr i8, i8* %current_cell_ptr, i32 1
  %target_cell_val = load i8, i8* %target_cell_ptr
  %additional_val = mul i8 %cell_value, 2
  %new_target_val = add i8 %target_cell_val, %additional_val
  store i8 %new_target_val, i8* %target_cell_ptr
  %target_cell_ptr1 = getelementptr i8, i8* %current_cell_ptr, i32 2
  %target_cell_val2 = load i8, i8* %target_cell_ptr1
  %additional_val3 = mul i8 %cell_value, 3
  %new_target_val4 = add i8 %target_cell_val2, %additional_val3
  store i8 %new_target_val4, i8* %target_cell_ptr1
  br label %multiply_after

multiply_after:                                   ; preds = %multiply_body, %after_init
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn set_initial_cell_values() {
    let instrs = vec![PointerIncrement {
        amount: 1,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![
                Wrapping(1),
                Wrapping(1),
                Wrapping(2),
                Wrapping(0),
                Wrapping(0),
                Wrapping(0),
            ],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 6)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 1, i32 2, i32 1, i1 true)
  %offset_cell_ptr1 = getelementptr i8, i8* %cells, i32 2
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr1, i8 2, i32 1, i32 1, i1 true)
  %offset_cell_ptr2 = getelementptr i8, i8* %cells, i32 3
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr2, i8 0, i32 3, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %new_cell_index = add i32 %cell_index, 1
  store i32 %new_cell_index, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_static_outputs() {
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &[],
        &ExecutionState {
            start_instr: None,
            cells: vec![],
            cell_ptr: 0,
            outputs: vec![5, 10],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

@known_outputs = constant [2 x i8] c\"\\05\\0A\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %0 = call i32 @write(i32 1, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @known_outputs, i32 0, i32 0), i32 2)
  br label %beginning

beginning:                                        ; preds = %init
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_ptr_increment() {
    let instrs = vec![PointerIncrement {
        amount: 1,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); 2],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 2)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 2, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %new_cell_index = add i32 %cell_index, 1
  store i32 %new_cell_index, i32* %cell_index_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_increment() {
    let instrs = vec![Increment {
        amount: Wrapping(1),
        offset: 0,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 1)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %offset_cell_index = add i32 %cell_index, 0
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %offset_cell_index
  %cell_value = load i8, i8* %current_cell_ptr
  %new_cell_value = add i8 %cell_value, 1
  store i8 %new_cell_value, i8* %current_cell_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_increment_with_offset() {
    let instrs = vec![Increment {
        amount: Wrapping(1),
        offset: 3,
        position: Some(Position { start: 0, end: 0 }),
    }];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); 4],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 4)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 4, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index = load i32, i32* %cell_index_ptr
  %offset_cell_index = add i32 %cell_index, 3
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %offset_cell_index
  %cell_value = load i8, i8* %current_cell_ptr
  %new_cell_value = add i8 %cell_value, 1
  store i8 %new_cell_value, i8* %current_cell_ptr
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";
    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

#[test]
fn compile_start_instr_midway() {
    let instrs = vec![
        Set {
            amount: Wrapping(1),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
        Set {
            amount: Wrapping(2),
            offset: 0,
            position: Some(Position { start: 0, end: 0 }),
        },
    ];
    let result = compile_to_module(
        "foo",
        Some("i686-pc-linux-gnu".to_owned()),
        &instrs,
        &ExecutionState {
            start_instr: Some(&instrs[1]),
            cells: vec![Wrapping(0)],
            cell_ptr: 0,
            outputs: vec![],
        },
    );
    let expected = "; ModuleID = \'foo\'
source_filename = \"foo\"
target triple = \"i686-pc-linux-gnu\"

; Function Attrs: argmemonly nounwind
declare void @llvm.memset.p0i8.i32(i8* nocapture writeonly, i8, i32, i32, i1) #0

declare i8* @malloc(i32)

declare void @free(i8*)

declare i32 @write(i32, i8*, i32)

declare i32 @putchar(i32)

declare i32 @getchar()

define i32 @main() {
init:
  %cells = call i8* @malloc(i32 1)
  %offset_cell_ptr = getelementptr i8, i8* %cells, i32 0
  call void @llvm.memset.p0i8.i32(i8* %offset_cell_ptr, i8 0, i32 1, i32 1, i1 true)
  %cell_index_ptr = alloca i32
  store i32 0, i32* %cell_index_ptr
  br label %after_init

beginning:                                        ; No predecessors!
  %cell_index = load i32, i32* %cell_index_ptr
  %offset_cell_index = add i32 %cell_index, 0
  %current_cell_ptr = getelementptr i8, i8* %cells, i32 %offset_cell_index
  store i8 1, i8* %current_cell_ptr
  br label %after_init

after_init:                                       ; preds = %init, %beginning
  %cell_index1 = load i32, i32* %cell_index_ptr
  %offset_cell_index2 = add i32 %cell_index1, 0
  %current_cell_ptr3 = getelementptr i8, i8* %cells, i32 %offset_cell_index2
  store i8 2, i8* %current_cell_ptr3
  call void @free(i8* %cells)
  ret i32 0
}

attributes #0 = { argmemonly nounwind }
";

    assert_cstring_eq!(result.to_cstring(), CString::new(expected).unwrap());
}

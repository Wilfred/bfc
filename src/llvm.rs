use llvm_sys::core::*;
use llvm_sys::{LLVMModule,LLVMBasicBlock,LLVMIntPredicate};
use llvm_sys::prelude::*;

use std::ffi::CString;

use bfir::Instruction;

const LLVM_FALSE: LLVMBool = 0;

/// Convert a Rust string to a C char pointer.
fn cstr(s: &str) -> *const i8 {
    let cstring = CString::new(s).unwrap();
    cstring.to_bytes_with_nul().as_ptr() as *const _
}

unsafe fn add_function(module: &mut LLVMModule, fn_name: &str,
                       args: &mut Vec<LLVMTypeRef>, ret_type: LLVMTypeRef) {
    let fn_type = 
        LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
    LLVMAddFunction(module, cstr(fn_name), fn_type);
}

unsafe fn add_c_declarations(module: &mut LLVMModule) {
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);

    add_function(
        module, "calloc",
        &mut vec![LLVMInt32Type(), LLVMInt32Type()], byte_pointer);
    
    add_function(
        module, "free",
        &mut vec![byte_pointer], LLVMVoidType());
    
    add_function(
        module, "putchar",
        &mut vec![LLVMInt32Type()], LLVMInt32Type());
    
    add_function(
        module, "getchar",
        &mut vec![], LLVMInt32Type());
}

unsafe fn add_function_call(module: &mut LLVMModule, bb: &mut LLVMBasicBlock,
                            fn_name: &str, args: &mut Vec<LLVMValueRef>,
                            name: &str) -> LLVMValueRef {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let function = LLVMGetNamedFunction(module, cstr(fn_name));

    let result = LLVMBuildCall(builder, function, args.as_mut_ptr(),
                               args.len() as u32, cstr(name));

    LLVMDisposeBuilder(builder);
    result
}

const NUM_CELLS: u64 = 30000;
const CELL_SIZE_IN_BYTES: u64 = 1;

unsafe fn add_cells_init(module: &mut LLVMModule, bb: &mut LLVMBasicBlock) -> LLVMValueRef {
    // calloc(30000, 1);
    let mut calloc_args = vec![
        LLVMConstInt(LLVMInt32Type(), NUM_CELLS, LLVM_FALSE),
        LLVMConstInt(LLVMInt32Type(), CELL_SIZE_IN_BYTES, LLVM_FALSE),
        ];
    add_function_call(module, bb, "calloc", &mut calloc_args, "cells")
}

unsafe fn create_module(module_name: &str) -> *mut LLVMModule {
    let module = LLVMModuleCreateWithName(cstr(module_name));
    add_c_declarations(&mut *module);

    module
}

/// Define up the main function and add preamble. Return the main
/// function and a reference to the cells and their current index.
unsafe fn add_main_init(module: &mut LLVMModule)
                        -> (LLVMValueRef, LLVMValueRef, LLVMValueRef) {
    let mut main_args = vec![];
    let main_type = LLVMFunctionType(
        LLVMInt32Type(), main_args.as_mut_ptr(), 0, LLVM_FALSE);
    let main_fn = LLVMAddFunction(module, cstr("main"),
                                  main_type);
    
    let bb = LLVMAppendBasicBlock(main_fn, cstr("entry"));
    let cells = add_cells_init(&mut *module, &mut *bb);

    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);
    
    // int cell_index = 0;
    let cell_index_ptr = LLVMBuildAlloca(
        builder, LLVMInt32Type(), cstr("cell_index_ptr"));
    let zero = LLVMConstInt(LLVMInt32Type(), 0, LLVM_FALSE);
    LLVMBuildStore(builder, zero, cell_index_ptr);

    LLVMDisposeBuilder(builder);

    (main_fn, cells, cell_index_ptr)
}

/// Add prologue to main function.
unsafe fn add_main_cleanup(module: &mut LLVMModule, main: LLVMValueRef, cells: LLVMValueRef) {
    let bb = LLVMGetLastBasicBlock(main);
    
    // free(cells);
    let mut free_args = vec![cells];
    add_function_call(module, &mut *bb, "free", &mut free_args, "");

    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let zero = LLVMConstInt(LLVMInt32Type(), 0, LLVM_FALSE);
    LLVMBuildRet(builder, zero);

    LLVMDisposeBuilder(builder);
}

unsafe fn compile_increment<'a>(amount: i32, bb: &'a mut LLVMBasicBlock,
                                cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                                -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, cstr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, cstr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder, current_cell_ptr, cstr("cell_value"));

    let increment_amount = LLVMConstInt(LLVMInt8Type(), amount as u64, LLVM_FALSE);
    let new_cell_val = LLVMBuildAdd(builder, cell_val, increment_amount,
                                    cstr("new_cell_value"));

    LLVMBuildStore(builder, new_cell_val, current_cell_ptr);

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_set<'a>(amount: i32, bb: &'a mut LLVMBasicBlock,
                          cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                          -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, cstr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, cstr("current_cell_ptr"));

    let new_cell_val = LLVMConstInt(LLVMInt8Type(), amount as u64, LLVM_FALSE);
    LLVMBuildStore(builder, new_cell_val, current_cell_ptr);

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_ptr_increment<'a>(amount: i32, bb: &'a mut LLVMBasicBlock,
                                    cell_index_ptr: LLVMValueRef)
                                    -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, cstr("cell_index"));

    let increment_amount = LLVMConstInt(LLVMInt32Type(), amount as u64, LLVM_FALSE);
    let new_cell_index = LLVMBuildAdd(builder, cell_index, increment_amount,
                                      cstr("new_cell_index"));

    LLVMBuildStore(builder, new_cell_index, cell_index_ptr);

    LLVMDisposeBuilder(builder);

    bb
}

unsafe fn compile_read<'a>(module: &mut LLVMModule, bb: &'a mut LLVMBasicBlock,
                           cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, cstr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, cstr("current_cell_ptr"));

    let mut getchar_args = vec![];
    let input_char = add_function_call(module, bb, "getchar", &mut getchar_args, "input_char");
    let input_byte = LLVMBuildTrunc(builder, input_char, LLVMInt8Type(),
                                    cstr("input_byte"));

    LLVMBuildStore(builder, input_byte, current_cell_ptr);

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_write<'a>(module: &mut LLVMModule, bb: &'a mut LLVMBasicBlock,
                            cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, cstr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder, cells, indices.as_mut_ptr(), indices.len() as u32,
        cstr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder, current_cell_ptr, cstr("cell_value"));

    let cell_val_as_char = LLVMBuildSExt(builder, cell_val, LLVMInt32Type(),
                                         cstr("cell_val_as_char"));
    
    let mut putchar_args = vec![cell_val_as_char];
    add_function_call(module, bb, "putchar", &mut putchar_args, "");

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_loop<'a>(module: &mut LLVMModule, bb: &'a mut LLVMBasicBlock,
                           loop_body: &Vec<Instruction>,
                           main_fn: LLVMValueRef,
                           cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();

    // First, we branch into the loop header from the previous basic
    // block.
    let loop_header = LLVMAppendBasicBlock(main_fn, cstr("loop_header"));
    LLVMPositionBuilderAtEnd(builder, bb);
    LLVMBuildBr(builder, loop_header);

    let mut loop_body_bb = LLVMAppendBasicBlock(main_fn, cstr("loop_body"));
    let loop_after = LLVMAppendBasicBlock(main_fn, cstr("loop_after"));

    // loop_header:
    //   %cell_value = ...
    //   %cell_value_is_zero = icmp ...
    //   br %cell_value_is_zero, %loop_after, %loop_body
    LLVMPositionBuilderAtEnd(builder, loop_header);
    // TODO: we do this several times, factor out duplication.
    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, cstr("cell_index"));
    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, cstr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder, current_cell_ptr, cstr("cell_value"));

    // TODO: factor out a function for this.
    let zero = LLVMConstInt(LLVMInt8Type(), 0, LLVM_FALSE);
    let cell_val_is_zero = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntEQ,
                                         zero, cell_val, cstr("cell_value_is_zero"));
    LLVMBuildCondBr(builder, cell_val_is_zero, loop_after, loop_body_bb);

    // Recursively compile instructions in the loop body.
    for instr in loop_body {
        loop_body_bb = compile_instr(instr, module, &mut *loop_body_bb, main_fn, cells,
                                     cell_index_ptr);
    }

    // When the loop is finished, jump back to the beginning of the
    // loop.
    LLVMPositionBuilderAtEnd(builder, loop_body_bb);
    LLVMBuildBr(builder, loop_header);

    LLVMDisposeBuilder(builder);
    &mut *loop_after
}

unsafe fn compile_instr<'a>(instr: &Instruction, module: &mut LLVMModule, bb: &'a mut LLVMBasicBlock,
                            main_fn: LLVMValueRef,
                            cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    match instr {
        &Instruction::Increment(amount) =>
            compile_increment(amount, bb, cells, cell_index_ptr),
        &Instruction::Set(amount) =>
            compile_set(amount, bb, cells, cell_index_ptr),
        &Instruction::PointerIncrement(amount) =>
            compile_ptr_increment(amount, bb, cell_index_ptr),
        &Instruction::Read =>
            compile_read(module, bb, cells, cell_index_ptr),
        &Instruction::Write =>
            compile_write(module, bb, cells, cell_index_ptr),
        &Instruction::Loop(ref body) => {
            compile_loop(module, bb, body, main_fn, cells, cell_index_ptr)
        }
    }
}

pub unsafe fn compile_to_ir(module_name: &str, instrs: &Vec<Instruction>) -> CString {
    let module = create_module(module_name);

    let (main_fn, cells, cell_index_ptr) = add_main_init(&mut *module);
    let mut bb = LLVMGetLastBasicBlock(main_fn);

    for instr in instrs {
        bb = compile_instr(instr, &mut *module, &mut *bb, main_fn,
                           cells, cell_index_ptr);
    }
    
    add_main_cleanup(&mut *module, main_fn, cells);
    
    let llvm_ir = LLVMPrintModuleToString(module);

    LLVMDisposeModule(module);

    CString::from_ptr(llvm_ir)
}

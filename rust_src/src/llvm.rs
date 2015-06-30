use llvm_sys::core::*;
use llvm_sys::{LLVMModule,LLVMBasicBlock};
use llvm_sys::prelude::*;

use std::ffi::CString;

const LLVM_FALSE: LLVMBool = 0;

unsafe fn add_function(module: &mut LLVMModule, fn_name: &str,
                       args: &mut Vec<LLVMTypeRef>, ret_type: LLVMTypeRef) {
    let fn_type = 
        LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
    let c_fn_name = CString::new(fn_name).unwrap();
    LLVMAddFunction(module, c_fn_name.to_bytes_with_nul().as_ptr() as *const _, fn_type);
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
    let context = LLVMGetGlobalContext();

    let builder = LLVMCreateBuilderInContext(context);
    LLVMPositionBuilderAtEnd(builder, bb);

    let c_fn_name = CString::new(fn_name).unwrap();
    let function = LLVMGetNamedFunction(
        module, c_fn_name.to_bytes_with_nul().as_ptr() as *const _);

    let c_name = CString::new(name).unwrap();
    let result = LLVMBuildCall(
        builder, function, args.as_mut_ptr(),
        args.len() as u32, c_name.to_bytes_with_nul().as_ptr() as *const _);

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
    let c_mod_name = CString::new(module_name).unwrap();

    let module = LLVMModuleCreateWithName(
        c_mod_name.to_bytes_with_nul().as_ptr() as *const _);
    add_c_declarations(&mut *module);

    module
}

/// Define up the main function and add preamble. Return the main
/// function and a reference to the cells.
unsafe fn add_main_init(module: &mut LLVMModule) -> (LLVMValueRef, LLVMValueRef) {
    let mut main_args = vec![];
    let main_type = LLVMFunctionType(
        LLVMInt32Type(), main_args.as_mut_ptr(), 0, LLVM_FALSE);
    let main_fn = LLVMAddFunction(module, b"main\0".as_ptr() as *const _,
                                  main_type);
    
    let context = LLVMGetGlobalContext();
    let bb = LLVMAppendBasicBlockInContext(
        context, main_fn, b"entry\0".as_ptr() as *const _);
    let cells = add_cells_init(&mut *module, &mut *bb);

    (main_fn, cells)
}

/// Add prologue to main function.
unsafe fn add_main_cleanup(module: &mut LLVMModule, main: LLVMValueRef, cells: LLVMValueRef) {
    let bb = LLVMGetLastBasicBlock(main);
    
    // free(cells);
    let mut free_args = vec![cells];
    add_function_call(module, &mut *bb, "free", &mut free_args, "");

    let context = LLVMGetGlobalContext();
    let builder = LLVMCreateBuilderInContext(context);
    LLVMPositionBuilderAtEnd(builder, bb);

    let five = LLVMConstInt(LLVMInt32Type(), 5, LLVM_FALSE);
    LLVMBuildRet(builder, five);

    LLVMDisposeBuilder(builder);
}

pub unsafe fn dump_ir(module_name: &str) -> CString {
    let module = create_module(module_name);

    let (main_fn, cells) = add_main_init(&mut *module);
    add_main_cleanup(&mut *module, main_fn, cells);
    
    let llvm_ir = LLVMPrintModuleToString(module);

    LLVMDisposeModule(module);

    CString::from_ptr(llvm_ir)
}

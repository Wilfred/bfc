use llvm_sys::core::*;
use llvm_sys::{LLVMModule,LLVMBasicBlock};
use llvm_sys::prelude::{LLVMTypeRef,LLVMValueRef};

use std::ffi::CString;

unsafe fn add_function(module: &mut LLVMModule, fn_name: &str,
                       args: &mut Vec<LLVMTypeRef>, ret_type: LLVMTypeRef) {
    let fn_type = 
        LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, 0);
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
                            name: &str) {
    let context = LLVMGetGlobalContext();

    let builder = LLVMCreateBuilderInContext(context);
    LLVMPositionBuilderAtEnd(builder, bb);

    let c_fn_name = CString::new(fn_name).unwrap();
    let function = LLVMGetNamedFunction(
        module, c_fn_name.to_bytes_with_nul().as_ptr() as *const _);

    let c_name = CString::new(name).unwrap();
    LLVMBuildCall(builder, function, args.as_mut_ptr(),
                  args.len() as u32, c_name.to_bytes_with_nul().as_ptr() as *const _);

    LLVMDisposeBuilder(builder);
}

const NUM_CELLS: u64 = 30000;
const CELL_SIZE_IN_BYTES: u64 = 1;

unsafe fn add_cells_init(module: &mut LLVMModule, bb: &mut LLVMBasicBlock) {
    // calloc(30000, 1);
    let mut calloc_args = vec![
        // TODO: define LLVM_FALSE as 0.
        LLVMConstInt(LLVMInt32Type(), NUM_CELLS, 0),
        LLVMConstInt(LLVMInt32Type(), CELL_SIZE_IN_BYTES, 0),
        ];
    add_function_call(module, bb, "calloc", &mut calloc_args, "cells");
}

pub unsafe fn dump_ir(module_name: &str) {
    let c_mod_name = CString::new(module_name).unwrap();
    
    let context = LLVMGetGlobalContext();
    let module = LLVMModuleCreateWithName(c_mod_name.to_bytes_with_nul().as_ptr() as *const _);

    add_c_declarations(&mut *module);

    let mut main_args = vec![];
    let main_type = LLVMFunctionType(
        LLVMInt32Type(), main_args.as_mut_ptr(), 0, 0);
    let main_fn = LLVMAddFunction(module, b"main\0".as_ptr() as *const _,
                                  main_type);

    let bb = LLVMAppendBasicBlockInContext(
        context, main_fn, b"entry\0".as_ptr() as *const _);
    add_cells_init(&mut *module, &mut *bb);
    
    let builder = LLVMCreateBuilderInContext(context);
    LLVMPositionBuilderAtEnd(builder, bb);
    LLVMBuildRetVoid(builder);

    // Dump the module as IR to stdout.
    LLVMDumpModule(module);

    LLVMDisposeBuilder(builder);
    LLVMDisposeModule(module);
}

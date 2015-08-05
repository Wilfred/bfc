use llvm_sys::core::*;
use llvm_sys::{LLVMModule,LLVMBasicBlock,LLVMIntPredicate};
use llvm_sys::prelude::*;

use libc::types::os::arch::c99::c_ulonglong;
use std::ffi::{CString,CStr};

use bfir::Instruction;
use bfir::Instruction::*;

const LLVM_FALSE: LLVMBool = 0;

/// A struct that keeps ownership of all the strings we've passed to
/// the LLVM API until we destroy the LLVMModule.
struct ModuleWithContext {
    module: *mut LLVMModule,
    strings: Vec<CString>
}

impl ModuleWithContext {
    /// Create a new CString associated with this LLVMModule,
    /// and return a pointer that can be passed to LLVM APIs.
    /// Assumes s is pure-ASCII.
    fn new_string_ptr(&mut self, s: &str) -> *const i8 {
        let cstring = CString::new(s).unwrap();
        let ptr = cstring.as_ptr() as *const _;
        self.strings.push(cstring);
        ptr
    }
}

unsafe fn add_function(module: &mut ModuleWithContext, fn_name: &str,
                       args: &mut Vec<LLVMTypeRef>, ret_type: LLVMTypeRef) {
    let fn_type = 
        LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
    LLVMAddFunction(module.module, module.new_string_ptr(fn_name), fn_type);
}

unsafe fn add_c_declarations(module: &mut ModuleWithContext) {
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);
    let void = LLVMVoidType();

    add_function(
        module, "malloc",
        &mut vec![LLVMInt32Type()], byte_pointer);

    // TODO: we should use memset for Set() commands.
    add_function(
        module, "llvm.memset.p0i8.i32",
        &mut vec![byte_pointer, LLVMInt8Type(), LLVMInt32Type(),
                  LLVMInt32Type(), LLVMInt1Type()],
        void);

    add_function(
        module, "free",
        &mut vec![byte_pointer], void);

    add_function(
        module, "putchar",
        &mut vec![LLVMInt32Type()], LLVMInt32Type());
    
    add_function(
        module, "getchar",
        &mut vec![], LLVMInt32Type());
}

unsafe fn add_function_call(module: &mut ModuleWithContext, bb: &mut LLVMBasicBlock,
                            fn_name: &str, args: &mut Vec<LLVMValueRef>,
                            name: &str) -> LLVMValueRef {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let function = LLVMGetNamedFunction(module.module, module.new_string_ptr(fn_name));

    let result = LLVMBuildCall(builder, function, args.as_mut_ptr(),
                               args.len() as u32, module.new_string_ptr(name));

    LLVMDisposeBuilder(builder);
    result
}

unsafe fn add_cells_init(num_cells: u64, module: &mut ModuleWithContext,
                         bb: &mut LLVMBasicBlock) -> LLVMValueRef {
    // malloc(30000);
    let llvm_num_cells = LLVMConstInt(LLVMInt32Type(), num_cells, LLVM_FALSE);
    let mut malloc_args = vec![llvm_num_cells];
    let cells = add_function_call(module, bb, "malloc", &mut malloc_args, "cells");

    let zero = LLVMConstInt(LLVMInt8Type(), 0, LLVM_FALSE);
    let one = LLVMConstInt(LLVMInt32Type(), 1, LLVM_FALSE);
    let false_ = LLVMConstInt(LLVMInt1Type(), 1, LLVM_FALSE);
    let mut memset_args = vec![
        // TODO: is one the correct alignment here? I've just blindly
        // copied from clang output.
        cells, zero, llvm_num_cells, one, false_];
    add_function_call(module, bb, "llvm.memset.p0i8.i32", &mut memset_args, "");

    cells
}

unsafe fn create_module(module_name: &str) -> ModuleWithContext {
    let c_module_name = CString::new(module_name).unwrap();
    
    let llvm_module = LLVMModuleCreateWithName(
        c_module_name.to_bytes_with_nul().as_ptr() as *const _);
    let mut module = ModuleWithContext { module: llvm_module, strings: vec![c_module_name] };
    add_c_declarations(&mut module);

    module
}

/// Define up the main function and add preamble. Return the main
/// function and a reference to the cells and their current index.
unsafe fn add_main_init(num_cells: u64, cell_ptr: i32, module: &mut ModuleWithContext)
                        -> (LLVMValueRef, LLVMValueRef, LLVMValueRef) {
    let mut main_args = vec![];
    let main_type = LLVMFunctionType(
        LLVMInt32Type(), main_args.as_mut_ptr(), 0, LLVM_FALSE);
    let main_fn = LLVMAddFunction(module.module, module.new_string_ptr("main"),
                                  main_type);
    
    let bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("entry"));
    let cells = add_cells_init(num_cells, module, &mut *bb);

    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);
    
    // int cell_index = 0;
    let cell_index_ptr = LLVMBuildAlloca(
        builder, LLVMInt32Type(), module.new_string_ptr("cell_index_ptr"));
    let cell_ptr_init = LLVMConstInt(LLVMInt32Type(), cell_ptr as c_ulonglong, LLVM_FALSE);
    LLVMBuildStore(builder, cell_ptr_init, cell_index_ptr);

    LLVMDisposeBuilder(builder);

    (main_fn, cells, cell_index_ptr)
}

/// Add prologue to main function.
unsafe fn add_main_cleanup(module: &mut ModuleWithContext, bb: &mut LLVMBasicBlock,
                           cells: LLVMValueRef) {
    // free(cells);
    let mut free_args = vec![cells];
    add_function_call(module, &mut *bb, "free", &mut free_args, "");

    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let zero = LLVMConstInt(LLVMInt32Type(), 0, LLVM_FALSE);
    LLVMBuildRet(builder, zero);

    LLVMDisposeBuilder(builder);
}

unsafe fn compile_increment<'a>(amount: u8, module: &mut ModuleWithContext, bb: &'a mut LLVMBasicBlock,
                                cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                                -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder, current_cell_ptr, module.new_string_ptr("cell_value"));

    let increment_amount = LLVMConstInt(LLVMInt8Type(), amount as u64, LLVM_FALSE);
    let new_cell_val = LLVMBuildAdd(builder, cell_val, increment_amount,
                                    module.new_string_ptr("new_cell_value"));

    LLVMBuildStore(builder, new_cell_val, current_cell_ptr);

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_set<'a>(amount: u8, module: &mut ModuleWithContext, bb: &'a mut LLVMBasicBlock,
                          cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                          -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));

    let new_cell_val = LLVMConstInt(LLVMInt8Type(), amount as u64, LLVM_FALSE);
    LLVMBuildStore(builder, new_cell_val, current_cell_ptr);

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_ptr_increment<'a>(amount: i32, module: &mut ModuleWithContext, bb: &'a mut LLVMBasicBlock,
                                    cell_index_ptr: LLVMValueRef)
                                    -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let increment_amount = LLVMConstInt(LLVMInt32Type(), amount as u64, LLVM_FALSE);
    let new_cell_index = LLVMBuildAdd(builder, cell_index, increment_amount,
                                      module.new_string_ptr("new_cell_index"));

    LLVMBuildStore(builder, new_cell_index, cell_index_ptr);

    LLVMDisposeBuilder(builder);

    bb
}

unsafe fn compile_read<'a>(module: &mut ModuleWithContext, bb: &'a mut LLVMBasicBlock,
                           cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));

    let mut getchar_args = vec![];
    let input_char = add_function_call(module, bb, "getchar", &mut getchar_args, "input_char");
    let input_byte = LLVMBuildTrunc(builder, input_char, LLVMInt8Type(),
                                    module.new_string_ptr("input_byte"));

    LLVMBuildStore(builder, input_byte, current_cell_ptr);

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_write<'a>(module: &mut ModuleWithContext, bb: &'a mut LLVMBasicBlock,
                            cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();
    LLVMPositionBuilderAtEnd(builder, bb);

    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder, cells, indices.as_mut_ptr(), indices.len() as u32,
        module.new_string_ptr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder, current_cell_ptr, module.new_string_ptr("cell_value"));

    let cell_val_as_char = LLVMBuildSExt(builder, cell_val, LLVMInt32Type(),
                                         module.new_string_ptr("cell_val_as_char"));
    
    let mut putchar_args = vec![cell_val_as_char];
    add_function_call(module, bb, "putchar", &mut putchar_args, "");

    LLVMDisposeBuilder(builder);
    bb
}

unsafe fn compile_loop<'a>(module: &mut ModuleWithContext, bb: &'a mut LLVMBasicBlock,
                           loop_body: &Vec<Instruction>,
                           main_fn: LLVMValueRef,
                           cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = LLVMCreateBuilder();

    // First, we branch into the loop header from the previous basic
    // block.
    let loop_header = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_header"));
    LLVMPositionBuilderAtEnd(builder, bb);
    LLVMBuildBr(builder, loop_header);

    let mut loop_body_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_body"));
    let loop_after = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_after"));

    // loop_header:
    //   %cell_value = ...
    //   %cell_value_is_zero = icmp ...
    //   br %cell_value_is_zero, %loop_after, %loop_body
    LLVMPositionBuilderAtEnd(builder, loop_header);
    // TODO: we do this several times, factor out duplication.
    let cell_index = LLVMBuildLoad(builder, cell_index_ptr, module.new_string_ptr("cell_index"));
    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder, current_cell_ptr, module.new_string_ptr("cell_value"));

    // TODO: factor out a function for this.
    let zero = LLVMConstInt(LLVMInt8Type(), 0, LLVM_FALSE);
    let cell_val_is_zero = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntEQ,
                                         zero, cell_val, module.new_string_ptr("cell_value_is_zero"));
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

unsafe fn compile_instr<'a>(instr: &Instruction, module: &mut ModuleWithContext,
                            bb: &'a mut LLVMBasicBlock, main_fn: LLVMValueRef,
                            cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    match instr {
        &Increment(amount) =>
            compile_increment(amount, module, bb, cells, cell_index_ptr),
        &Set(amount) =>
            compile_set(amount, module, bb, cells, cell_index_ptr),
        &PointerIncrement(amount) =>
            compile_ptr_increment(amount, module, bb, cell_index_ptr),
        &Read =>
            compile_read(module, bb, cells, cell_index_ptr),
        &Write =>
            compile_write(module, bb, cells, cell_index_ptr),
        &Loop(ref body) => {
            compile_loop(module, bb, body, main_fn, cells, cell_index_ptr)
        }
    }
}

pub fn compile_to_ir(module_name: &str, instrs: &Vec<Instruction>,
                     num_cells: u64, cell_ptr: i32) -> CString {
    let llvm_ir_owned;
    unsafe {
        let mut module = create_module(module_name);

        let (main_fn, cells, cell_index_ptr) = add_main_init(num_cells, cell_ptr, &mut module);
        let mut bb = LLVMGetLastBasicBlock(main_fn);

        // TODO: don't bother with init/cleanup if we have an empty
        // program.
        for instr in instrs {
            bb = compile_instr(instr, &mut module, &mut *bb, main_fn,
                               cells, cell_index_ptr);
        }
        
        add_main_cleanup(&mut module, &mut *bb, cells);

        // LLVM gives us a *char pointer, so wrap it in a CStr to mark it
        // as borrowed.
        let llvm_ir_ptr = LLVMPrintModuleToString(module.module);
        let llvm_ir = CStr::from_ptr(llvm_ir_ptr);

        // Make an owned copy of the string in our memory space.
        llvm_ir_owned = CString::new(llvm_ir.to_bytes().clone()).unwrap();

        // Cleanup module and borrowed string.
        LLVMDisposeModule(module.module);
        LLVMDisposeMessage(llvm_ir_ptr);
    }

    llvm_ir_owned
}

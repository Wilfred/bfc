use itertools::Itertools;
use llvm_sys::core::*;
use llvm_sys::{LLVMModule,LLVMBasicBlock,LLVMIntPredicate,LLVMBuilder};
use llvm_sys::prelude::*;

use libc::types::os::arch::c99::c_ulonglong;
use libc::types::os::arch::c95::c_uint;
use std::ffi::{CString,CStr};

use bfir::Instruction;
use bfir::Instruction::*;

const LLVM_FALSE: LLVMBool = 0;
const LLVM_TRUE: LLVMBool = 1;

/// A struct that keeps ownership of all the strings we've passed to
/// the LLVM API until we destroy the LLVMModule.
struct Module {
    module: *mut LLVMModule,
    strings: Vec<CString>
}

impl Module {
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

/// Wraps LLVM's builder class to provide a nicer API and ensure we
/// always dispose correctly.
struct Builder {
    builder: *mut LLVMBuilder
}

impl Builder {
    /// Create a new Builder in LLVM's global context.
    unsafe fn new() -> Self {
        Builder { builder: LLVMCreateBuilder() }
    }

    unsafe fn position_at_end(&self, bb: *mut LLVMBasicBlock) {
        LLVMPositionBuilderAtEnd(self.builder, bb);
    }
}

impl Drop for Builder {
    fn drop(&mut self) {
        // Rust requires that drop() is a safe function.
        unsafe {
            LLVMDisposeBuilder(self.builder);
        }
    }
}

unsafe fn add_function(module: &mut Module, fn_name: &str,
                       args: &mut Vec<LLVMTypeRef>, ret_type: LLVMTypeRef) {
    let fn_type = 
        LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
    LLVMAddFunction(module.module, module.new_string_ptr(fn_name), fn_type);
}

unsafe fn add_c_declarations(module: &mut Module) {
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);
    let void = LLVMVoidType();

    // TODO: we should use memset for Set() commands.
    add_function(
        module, "llvm.memset.p0i8.i32",
        &mut vec![byte_pointer, LLVMInt8Type(), LLVMInt32Type(),
                  LLVMInt32Type(), LLVMInt1Type()],
        void);

    add_function(
        module, "write",
        &mut vec![LLVMInt32Type(), byte_pointer, LLVMInt32Type()], LLVMInt32Type());
    
    add_function(
        module, "putchar",
        &mut vec![LLVMInt32Type()], LLVMInt32Type());
    
    add_function(
        module, "getchar",
        &mut vec![], LLVMInt32Type());
}

// TODO: take slice here rather than Vec.
unsafe fn add_function_call(module: &mut Module, bb: &mut LLVMBasicBlock,
                            fn_name: &str, args: &mut Vec<LLVMValueRef>,
                            name: &str) -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let function = LLVMGetNamedFunction(module.module, module.new_string_ptr(fn_name));

    // TODO: use c_uint here
    LLVMBuildCall(builder.builder, function, args.as_mut_ptr(),
                  args.len() as u32, module.new_string_ptr(name))
}

/// Given a vector of cells [1, 1, 0, 0, 0, ...] return a vector
/// [(1, 2), (0, 3), ...].
fn run_length_encode(cells: &[u8]) -> Vec<(u8, usize)> {
    cells.into_iter().map(|val| {
        (*val, 1)
    }).coalesce(|(prev_val, prev_count), (val, count)| {
        if prev_val == val {
            Ok((val, prev_count + count))
        } else {
            Err(((prev_val, prev_count), (val, count)))
        }
    }).collect()
}

unsafe fn add_cells_init(init_values: &[u8], module: &mut Module,
                         bb: &mut LLVMBasicBlock) -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    // Allocate stack memory for our cells.
    let num_cells = LLVMConstInt(LLVMInt32Type(), init_values.len() as c_ulonglong, LLVM_FALSE);
    let cells_ptr = LLVMBuildArrayAlloca(builder.builder, LLVMInt8Type(), num_cells,
                                         module.new_string_ptr("cells"));

    let one = LLVMConstInt(LLVMInt32Type(), 1, LLVM_FALSE);
    let false_ = LLVMConstInt(LLVMInt1Type(), 1, LLVM_FALSE);

    let mut offset = 0;
    for (cell_val, cell_count) in run_length_encode(init_values) {
        let llvm_cell_val = LLVMConstInt(LLVMInt8Type(), cell_val as c_ulonglong, LLVM_FALSE);
        let llvm_cell_count = LLVMConstInt(LLVMInt32Type(), cell_count as c_ulonglong, LLVM_FALSE);

        // TODO: factor out a build_gep function.
        let mut offset_vec = vec![LLVMConstInt(LLVMInt32Type(), offset as c_ulonglong, LLVM_FALSE)];
        let offset_cell_ptr = LLVMBuildGEP(builder.builder, cells_ptr, offset_vec.as_mut_ptr(),
                                           offset_vec.len() as u32, module.new_string_ptr("offset_cell_ptr"));
        
        let mut memset_args = vec![
            // TODO: is one the correct alignment here? I've just blindly
            // copied from clang output.
            offset_cell_ptr, llvm_cell_val, llvm_cell_count, one, false_];
        add_function_call(module, bb, "llvm.memset.p0i8.i32", &mut memset_args, "");

        offset += cell_count;
    }

    cells_ptr
}

unsafe fn create_module(module_name: &str) -> Module {
    let c_module_name = CString::new(module_name).unwrap();
    
    let llvm_module = LLVMModuleCreateWithName(
        c_module_name.to_bytes_with_nul().as_ptr() as *const _);
    let mut module = Module { module: llvm_module, strings: vec![c_module_name] };
    add_c_declarations(&mut module);

    module
}

unsafe fn add_main_fn(module: &mut Module) -> LLVMValueRef {
    let mut main_args = vec![];
    let main_type = LLVMFunctionType(
        LLVMInt32Type(), main_args.as_mut_ptr(), 0, LLVM_FALSE);
    let main_fn = LLVMAddFunction(module.module, module.new_string_ptr("main"),
                                  main_type);
    
    LLVMAppendBasicBlock(main_fn, module.new_string_ptr("entry"));
    main_fn
}

/// Initialise the value that contains the current cell index.
unsafe fn add_cell_index_init(init_value: i32, bb: *mut LLVMBasicBlock,
                              module: &mut Module) -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    // int cell_index = 0;
    let cell_index_ptr = LLVMBuildAlloca(
        builder.builder, LLVMInt32Type(), module.new_string_ptr("cell_index_ptr"));
    let cell_ptr_init = LLVMConstInt(LLVMInt32Type(), init_value as c_ulonglong, LLVM_FALSE);
    LLVMBuildStore(builder.builder, cell_ptr_init, cell_index_ptr);

    cell_index_ptr
}

/// Add prologue to main function.
unsafe fn add_main_cleanup(bb: *mut LLVMBasicBlock) {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    let zero = LLVMConstInt(LLVMInt32Type(), 0, LLVM_FALSE);
    LLVMBuildRet(builder.builder, zero);
}

unsafe fn compile_increment<'a>(amount: u8, module: &mut Module, bb: &'a mut LLVMBasicBlock,
                                cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                                -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    let cell_index = LLVMBuildLoad(builder.builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder.builder, current_cell_ptr, module.new_string_ptr("cell_value"));

    let increment_amount = LLVMConstInt(LLVMInt8Type(), amount as u64, LLVM_FALSE);
    let new_cell_val = LLVMBuildAdd(builder.builder, cell_val, increment_amount,
                                    module.new_string_ptr("new_cell_value"));

    LLVMBuildStore(builder.builder, new_cell_val, current_cell_ptr);
    bb
}

unsafe fn compile_set<'a>(amount: u8, module: &mut Module, bb: &'a mut LLVMBasicBlock,
                          cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                          -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    let cell_index = LLVMBuildLoad(builder.builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));

    let new_cell_val = LLVMConstInt(LLVMInt8Type(), amount as u64, LLVM_FALSE);
    LLVMBuildStore(builder.builder, new_cell_val, current_cell_ptr);
    bb
}

unsafe fn compile_ptr_increment<'a>(amount: i32, module: &mut Module, bb: &'a mut LLVMBasicBlock,
                                    cell_index_ptr: LLVMValueRef)
                                    -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    let cell_index = LLVMBuildLoad(builder.builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let increment_amount = LLVMConstInt(LLVMInt32Type(), amount as u64, LLVM_FALSE);
    let new_cell_index = LLVMBuildAdd(builder.builder, cell_index, increment_amount,
                                      module.new_string_ptr("new_cell_index"));

    LLVMBuildStore(builder.builder, new_cell_index, cell_index_ptr);
    bb
}

unsafe fn compile_read<'a>(module: &mut Module, bb: &'a mut LLVMBasicBlock,
                           cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    let cell_index = LLVMBuildLoad(builder.builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));

    let mut getchar_args = vec![];
    let input_char = add_function_call(module, bb, "getchar", &mut getchar_args, "input_char");
    let input_byte = LLVMBuildTrunc(builder.builder, input_char, LLVMInt8Type(),
                                    module.new_string_ptr("input_byte"));

    LLVMBuildStore(builder.builder, input_byte, current_cell_ptr);
    bb
}

unsafe fn compile_write<'a>(module: &mut Module, bb: &'a mut LLVMBasicBlock,
                            cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);
    
    let cell_index = LLVMBuildLoad(builder.builder, cell_index_ptr, module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder.builder, cells, indices.as_mut_ptr(), indices.len() as u32,
        module.new_string_ptr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder.builder, current_cell_ptr, module.new_string_ptr("cell_value"));

    let cell_val_as_char = LLVMBuildSExt(builder.builder, cell_val, LLVMInt32Type(),
                                         module.new_string_ptr("cell_val_as_char"));
    
    let mut putchar_args = vec![cell_val_as_char];
    add_function_call(module, bb, "putchar", &mut putchar_args, "");
    bb
}

unsafe fn compile_loop<'a>(module: &mut Module, bb: &'a mut LLVMBasicBlock,
                           loop_body: &[Instruction],
                           main_fn: LLVMValueRef,
                           cells: LLVMValueRef, cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    
    // First, we branch into the loop header from the previous basic
    // block.
    let loop_header = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_header"));
    builder.position_at_end(bb);
    LLVMBuildBr(builder.builder, loop_header);

    let mut loop_body_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_body"));
    let loop_after = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_after"));

    // loop_header:
    //   %cell_value = ...
    //   %cell_value_is_zero = icmp ...
    //   br %cell_value_is_zero, %loop_after, %loop_body
    builder.position_at_end(loop_header);
    // TODO: we do this several times, factor out duplication.
    let cell_index = LLVMBuildLoad(builder.builder, cell_index_ptr, module.new_string_ptr("cell_index"));
    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder, cells, indices.as_mut_ptr(),
                                        indices.len() as u32, module.new_string_ptr("current_cell_ptr"));
    let cell_val = LLVMBuildLoad(builder.builder, current_cell_ptr, module.new_string_ptr("cell_value"));

    // TODO: factor out a function for this.
    let zero = LLVMConstInt(LLVMInt8Type(), 0, LLVM_FALSE);
    let cell_val_is_zero = LLVMBuildICmp(builder.builder, LLVMIntPredicate::LLVMIntEQ,
                                         zero, cell_val, module.new_string_ptr("cell_value_is_zero"));
    LLVMBuildCondBr(builder.builder, cell_val_is_zero, loop_after, loop_body_bb);

    // Recursively compile instructions in the loop body.
    for instr in loop_body {
        loop_body_bb = compile_instr(instr, module, &mut *loop_body_bb, main_fn, cells,
                                     cell_index_ptr);
    }

    // When the loop is finished, jump back to the beginning of the
    // loop.
    builder.position_at_end(loop_body_bb);
    LLVMBuildBr(builder.builder, loop_header);

    &mut *loop_after
}

// TODO: just take * instead of & to save all the casting.
unsafe fn compile_instr<'a>(instr: &Instruction, module: &mut Module,
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

unsafe fn compile_static_outputs(module: &mut Module,
                                 bb: &mut LLVMBasicBlock, outputs: &[u8]) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let mut llvm_outputs = vec![];
    for value in outputs {
        llvm_outputs.push(
            LLVMConstInt(LLVMInt8Type(), *value as c_ulonglong, LLVM_FALSE));
    }

    let output_buf_type = LLVMArrayType(LLVMInt8Type(), llvm_outputs.len() as c_uint);
    let llvm_outputs_arr = LLVMConstArray(LLVMInt8Type(), llvm_outputs.as_mut_ptr(),
                                          llvm_outputs.len() as c_uint);

    let known_outputs = LLVMAddGlobal(module.module, output_buf_type, module.new_string_ptr("known_outputs"));
    // TODO: define a utility function for constant integers.
    LLVMSetInitializer(known_outputs, llvm_outputs_arr);
    LLVMSetGlobalConstant(known_outputs, LLVM_TRUE);

    let stdout_fd = LLVMConstInt(LLVMInt32Type(), 0, LLVM_FALSE);
    let llvm_num_outputs = LLVMConstInt(LLVMInt32Type(), outputs.len() as c_ulonglong, LLVM_FALSE);

    // TODO: worth factoring out this type too.
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);
    let known_outputs_ptr = LLVMBuildPointerCast(builder.builder, known_outputs, byte_pointer, module.new_string_ptr("known_outputs_ptr"));

    add_function_call(module, bb, "write", &mut vec![stdout_fd, known_outputs_ptr, llvm_num_outputs], "");
}

// TODO: take a compile state rather than passing tons of variables.
// TODO: use init_values terminology consistently for names here.
pub fn compile_to_ir(module_name: &str, instrs: &[Instruction],
                     cells: &[u8], cell_ptr: i32, static_outputs: &[u8])
                     -> CString {
    let llvm_ir_owned;
    unsafe {
        let mut module = create_module(module_name);

        let main_fn = add_main_fn(&mut module);
        let mut bb = LLVMGetLastBasicBlock(main_fn);

        if static_outputs.len() > 0 {
            compile_static_outputs(&mut module, &mut *bb, static_outputs);
        }

        if instrs.len() > 0 {
            // TODO: decide on a consistent order between module and bb as
            // parameters.
            let llvm_cells = add_cells_init(cells, &mut module, &mut *bb);
            let llvm_cell_index = add_cell_index_init(cell_ptr, bb, &mut module);

            for instr in instrs {
                bb = compile_instr(instr, &mut module, &mut *bb, main_fn,
                                   llvm_cells, llvm_cell_index);
            }
        }
        
        add_main_cleanup(bb);

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

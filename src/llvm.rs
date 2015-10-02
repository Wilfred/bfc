
use itertools::Itertools;
use llvm_sys::core::*;
use llvm_sys::{LLVMModule, LLVMBasicBlock, LLVMIntPredicate, LLVMBuilder};
use llvm_sys::prelude::*;

use libc::types::os::arch::c99::c_ulonglong;
use libc::types::os::arch::c95::c_uint;
use std::ffi::{CString, CStr};

use std::collections::HashMap;
use std::num::Wrapping;

use bfir::{Instruction, Cell};
use bfir::Instruction::*;

use execution::ExecutionState;

const LLVM_FALSE: LLVMBool = 0;
const LLVM_TRUE: LLVMBool = 1;

/// A struct that keeps ownership of all the strings we've passed to
/// the LLVM API until we destroy the LLVMModule.
struct Module {
    module: *mut LLVMModule,
    strings: Vec<CString>,
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
    builder: *mut LLVMBuilder,
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

#[derive(Clone)]
struct CompileContext {
    cells: LLVMValueRef
}

/// Convert this integer to LLVM's representation of a constant
/// integer.
unsafe fn int8(val: c_ulonglong) -> LLVMValueRef {
    LLVMConstInt(LLVMInt8Type(), val, LLVM_FALSE)
}
/// Convert this integer to LLVM's representation of a constant
/// integer.
unsafe fn int32(val: c_ulonglong) -> LLVMValueRef {
    LLVMConstInt(LLVMInt32Type(), val, LLVM_FALSE)
}

unsafe fn add_function(module: &mut Module,
                       fn_name: &str,
                       args: &mut [LLVMTypeRef],
                       ret_type: LLVMTypeRef) {
    let fn_type =
        LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
    LLVMAddFunction(module.module, module.new_string_ptr(fn_name), fn_type);
}

unsafe fn add_c_declarations(module: &mut Module) {
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);
    let void = LLVMVoidType();

    add_function(module,
                 "llvm.memset.p0i8.i32",
                 &mut vec![byte_pointer, LLVMInt8Type(), LLVMInt32Type(),
                  LLVMInt32Type(), LLVMInt1Type()],
                 void);

    add_function(module,
                 "write",
                 &mut vec![LLVMInt32Type(), byte_pointer, LLVMInt32Type()],
                 LLVMInt32Type());

    add_function(module, "putchar", &mut vec![LLVMInt32Type()], LLVMInt32Type());

    add_function(module, "getchar", &mut vec![], LLVMInt32Type());
}

unsafe fn add_function_call(module: &mut Module,
                            bb: &mut LLVMBasicBlock,
                            fn_name: &str,
                            args: &mut [LLVMValueRef],
                            name: &str)
                            -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let function = LLVMGetNamedFunction(module.module, module.new_string_ptr(fn_name));

    LLVMBuildCall(builder.builder,
                  function,
                  args.as_mut_ptr(),
                  args.len() as c_uint,
                  module.new_string_ptr(name))
}

/// Given a vector of cells [1, 1, 0, 0, 0, ...] return a vector
/// [(1, 2), (0, 3), ...].
fn run_length_encode<T>(cells: &[T]) -> Vec<(T, usize)>
    where T: Eq + Copy {
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

unsafe fn add_cells_init(init_values: &[Wrapping<i8>],
                         module: &mut Module,
                         bb: &mut LLVMBasicBlock)
                         -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    // Allocate stack memory for our cells.
    let num_cells = int32(init_values.len() as c_ulonglong);
    let cells_ptr = LLVMBuildArrayAlloca(builder.builder,
                                         LLVMInt8Type(),
                                         num_cells,
                                         module.new_string_ptr("cells"));

    let one = int32(1);
    let false_ = LLVMConstInt(LLVMInt1Type(), 1, LLVM_FALSE);

    let mut offset = 0;
    for (cell_val, cell_count) in run_length_encode(init_values) {
        let llvm_cell_val = int8(cell_val.0 as c_ulonglong);
        let llvm_cell_count = int32(cell_count as c_ulonglong);

        // TODO: factor out a build_gep function.
        let mut offset_vec = vec![int32(offset as c_ulonglong)];
        let offset_cell_ptr = LLVMBuildGEP(builder.builder,
                                           cells_ptr,
                                           offset_vec.as_mut_ptr(),
                                           offset_vec.len() as u32,
                                           module.new_string_ptr("offset_cell_ptr"));

        let mut memset_args = vec![
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
    let main_type = LLVMFunctionType(LLVMInt32Type(), main_args.as_mut_ptr(), 0, LLVM_FALSE);
    let main_fn = LLVMAddFunction(module.module, module.new_string_ptr("main"), main_type);

    LLVMAppendBasicBlock(main_fn, module.new_string_ptr("entry"));
    main_fn
}

// TODO: name our pointers cell_base and
// cell_offset_ptr.
/// Initialise the value that contains the current cell index.
unsafe fn add_cell_index_init(init_value: isize,
                              bb: *mut LLVMBasicBlock,
                              module: &mut Module)
                              -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    // int cell_index = 0;
    let cell_index_ptr = LLVMBuildAlloca(builder.builder,
                                         LLVMInt32Type(),
                                         module.new_string_ptr("cell_index_ptr"));
    let cell_ptr_init = int32(init_value as c_ulonglong);
    LLVMBuildStore(builder.builder, cell_ptr_init, cell_index_ptr);

    cell_index_ptr
}

/// Add prologue to main function.
unsafe fn add_main_cleanup(bb: *mut LLVMBasicBlock) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let zero = int32(0);
    LLVMBuildRet(builder.builder, zero);
}

/// Add LLVM IR instructions for accessing the current cell, and
/// return a reference to the current cell, and to a current cell pointer.
unsafe fn add_current_cell_access(module: &mut Module,
                                  bb: &mut LLVMBasicBlock,
                                  cells: LLVMValueRef,
                                  cell_index_ptr: LLVMValueRef)
                                  -> (LLVMValueRef, LLVMValueRef) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(builder.builder,
                                   cell_index_ptr,
                                   module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder,
                                        cells,
                                        indices.as_mut_ptr(),
                                        indices.len() as u32,
                                        module.new_string_ptr("current_cell_ptr"));
    let current_cell = LLVMBuildLoad(builder.builder,
                                     current_cell_ptr,
                                     module.new_string_ptr("cell_value"));

    (current_cell, current_cell_ptr)
}

unsafe fn compile_increment<'a>(amount: Cell,
                                offset: isize,
                                module: &mut Module,
                                bb: &'a mut LLVMBasicBlock,
                                cells: LLVMValueRef,
                                cell_index_ptr: LLVMValueRef)
                                -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(builder.builder,
                                   cell_index_ptr,
                                   module.new_string_ptr("cell_index"));

    let offset_cell_index = LLVMBuildAdd(builder.builder,
                                         cell_index,
                                         int32(offset as c_ulonglong),
                                         module.new_string_ptr("offset_cell_index"));

    let mut indices = vec![offset_cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder,
                                        cells,
                                        indices.as_mut_ptr(),
                                        indices.len() as c_uint,
                                        module.new_string_ptr("current_cell_ptr"));

    let cell_val = LLVMBuildLoad(builder.builder,
                                 current_cell_ptr,
                                 module.new_string_ptr("cell_value"));

    let increment_amount = int8(amount.0 as c_ulonglong);
    let new_cell_val = LLVMBuildAdd(builder.builder,
                                    cell_val,
                                    increment_amount,
                                    module.new_string_ptr("new_cell_value"));

    LLVMBuildStore(builder.builder, new_cell_val, current_cell_ptr);
    bb
}

unsafe fn compile_set<'a>(amount: Cell,
                          offset: isize,
                          module: &mut Module,
                          bb: &'a mut LLVMBasicBlock,
                          cells: LLVMValueRef,
                          cell_index_ptr: LLVMValueRef)
                          -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(builder.builder,
                                   cell_index_ptr,
                                   module.new_string_ptr("cell_index"));

    let offset_cell_index = LLVMBuildAdd(builder.builder,
                                         cell_index,
                                         int32(offset as c_ulonglong),
                                         module.new_string_ptr("offset_cell_index"));

    let mut indices = vec![offset_cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder,
                                        cells,
                                        indices.as_mut_ptr(),
                                        indices.len() as c_uint,
                                        module.new_string_ptr("current_cell_ptr"));

    LLVMBuildStore(builder.builder, int8(amount.0 as c_ulonglong), current_cell_ptr);
    bb
}

unsafe fn compile_multiply_move<'a>(changes: &HashMap<isize, Cell>,
                                    module: &mut Module,
                                    bb: &'a mut LLVMBasicBlock,
                                    cells: LLVMValueRef,
                                    cell_index_ptr: LLVMValueRef)
                                    -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);

    // First, get the current cell value.
    let (cell_val, cell_val_ptr) = add_current_cell_access(module, bb, cells, cell_index_ptr);

    // Zero the current cell.
    LLVMBuildStore(builder.builder, int8(0), cell_val_ptr);

    let mut targets: Vec<_> = changes.keys().collect();
    targets.sort();

    // For each cell that we should change, multiply the current cell
    // value then add it.
    for target in targets {
        // Calculate the position of this target cell.
        let mut indices = vec![int32(*target as c_ulonglong)];
        let target_cell_ptr = LLVMBuildGEP(builder.builder,
                                           cell_val_ptr,
                                           indices.as_mut_ptr(),
                                           indices.len() as c_uint,
                                           module.new_string_ptr("target_cell_ptr"));

        // Get the current value of the current cell.
        let target_cell_val = LLVMBuildLoad(builder.builder,
                                            target_cell_ptr,
                                            module.new_string_ptr("target_cell_val"));

        // Calculate the new value.
        let factor = *changes.get(target).unwrap();
        let additional_val = LLVMBuildMul(builder.builder,
                                          cell_val,
                                          int8(factor.0 as c_ulonglong),
                                          module.new_string_ptr("additional_val"));
        let new_target_val = LLVMBuildAdd(builder.builder,
                                          target_cell_val,
                                          additional_val,
                                          module.new_string_ptr("new_target_val"));
        LLVMBuildStore(builder.builder, new_target_val, target_cell_ptr);
    }

    bb
}

unsafe fn compile_ptr_increment<'a>(amount: isize,
                                    module: &mut Module,
                                    bb: &'a mut LLVMBasicBlock,
                                    cell_index_ptr: LLVMValueRef)
                                    -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(builder.builder,
                                   cell_index_ptr,
                                   module.new_string_ptr("cell_index"));

    let new_cell_index = LLVMBuildAdd(builder.builder,
                                      cell_index,
                                      int32(amount as c_ulonglong),
                                      module.new_string_ptr("new_cell_index"));

    LLVMBuildStore(builder.builder, new_cell_index, cell_index_ptr);
    bb
}

unsafe fn compile_read<'a>(module: &mut Module,
                           bb: &'a mut LLVMBasicBlock,
                           cells: LLVMValueRef,
                           cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(builder.builder,
                                   cell_index_ptr,
                                   module.new_string_ptr("cell_index"));

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(builder.builder,
                                        cells,
                                        indices.as_mut_ptr(),
                                        indices.len() as u32,
                                        module.new_string_ptr("current_cell_ptr"));

    let mut getchar_args = vec![];
    let input_char = add_function_call(module, bb, "getchar", &mut getchar_args, "input_char");
    let input_byte = LLVMBuildTrunc(builder.builder,
                                    input_char,
                                    LLVMInt8Type(),
                                    module.new_string_ptr("input_byte"));

    LLVMBuildStore(builder.builder, input_byte, current_cell_ptr);
    bb
}

unsafe fn compile_write<'a>(module: &mut Module,
                            bb: &'a mut LLVMBasicBlock,
                            cells: LLVMValueRef,
                            cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_val = add_current_cell_access(module, bb, cells, cell_index_ptr).0;
    let cell_val_as_char = LLVMBuildSExt(builder.builder,
                                         cell_val,
                                         LLVMInt32Type(),
                                         module.new_string_ptr("cell_val_as_char"));

    let mut putchar_args = vec![cell_val_as_char];
    add_function_call(module, bb, "putchar", &mut putchar_args, "");
    bb
}

unsafe fn compile_loop<'a>(module: &mut Module,
                           bb: &'a mut LLVMBasicBlock,
                           loop_body: &[Instruction],
                           main_fn: LLVMValueRef,
                           cells: LLVMValueRef,
                           cell_index_ptr: LLVMValueRef)
                           -> &'a mut LLVMBasicBlock {
    let builder = Builder::new();

    // First, we branch into the loop header from the previous basic
    // block.
    let loop_header_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_header"));
    builder.position_at_end(bb);
    LLVMBuildBr(builder.builder, loop_header_bb);

    let mut loop_body_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_body"));
    let loop_after = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("loop_after"));

    // loop_header:
    //   %cell_value = ...
    //   %cell_value_is_zero = icmp ...
    //   br %cell_value_is_zero, %loop_after, %loop_body
    builder.position_at_end(loop_header_bb);

    let cell_val = add_current_cell_access(module, &mut *loop_header_bb, cells, cell_index_ptr).0;

    let zero = int8(0);
    let cell_val_is_zero = LLVMBuildICmp(builder.builder,
                                         LLVMIntPredicate::LLVMIntEQ,
                                         zero,
                                         cell_val,
                                         module.new_string_ptr("cell_value_is_zero"));
    LLVMBuildCondBr(builder.builder, cell_val_is_zero, loop_after, loop_body_bb);

    // Recursively compile instructions in the loop body.
    for instr in loop_body {
        loop_body_bb = compile_instr(instr, module, &mut *loop_body_bb, main_fn, cells,
                                     cell_index_ptr);
    }

    // When the loop is finished, jump back to the beginning of the
    // loop.
    builder.position_at_end(loop_body_bb);
    LLVMBuildBr(builder.builder, loop_header_bb);

    &mut *loop_after
}

unsafe fn compile_instr<'a>(instr: &Instruction,
                            module: &mut Module,
                            bb: &'a mut LLVMBasicBlock,
                            main_fn: LLVMValueRef,
                            cells: LLVMValueRef,
                            cell_index_ptr: LLVMValueRef)
                            -> &'a mut LLVMBasicBlock {
    match *instr {
        Increment { amount, offset } => {
            compile_increment(amount, offset, module, bb, cells, cell_index_ptr)
        },
        Set { amount, offset } => {
            compile_set(amount, offset, module, bb, cells, cell_index_ptr)
        },
        MultiplyMove(ref changes) => {
            compile_multiply_move(changes, module, bb, cells, cell_index_ptr)
        }
        PointerIncrement(amount) => compile_ptr_increment(amount, module, bb, cell_index_ptr),
        Read => compile_read(module, bb, cells, cell_index_ptr),
        Write => compile_write(module, bb, cells, cell_index_ptr),
        Loop(ref body) => {
            // TODO: we should pass arguments in a consistent order.
            compile_loop(module, bb, body, main_fn, cells, cell_index_ptr)
        }
    }
}

unsafe fn compile_static_outputs(module: &mut Module, bb: &mut LLVMBasicBlock, outputs: &[i8]) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let mut llvm_outputs = vec![];
    for value in outputs {
        llvm_outputs.push(int8(*value as c_ulonglong));
    }

    let output_buf_type = LLVMArrayType(LLVMInt8Type(), llvm_outputs.len() as c_uint);
    let llvm_outputs_arr = LLVMConstArray(LLVMInt8Type(),
                                          llvm_outputs.as_mut_ptr(),
                                          llvm_outputs.len() as c_uint);

    let known_outputs = LLVMAddGlobal(module.module,
                                      output_buf_type,
                                      module.new_string_ptr("known_outputs"));
    LLVMSetInitializer(known_outputs, llvm_outputs_arr);
    LLVMSetGlobalConstant(known_outputs, LLVM_TRUE);

    let stdout_fd = int32(1);
    let llvm_num_outputs = int32(outputs.len() as c_ulonglong);

    // TODO: worth factoring out this type too.
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);
    let known_outputs_ptr = LLVMBuildPointerCast(builder.builder,
                                                 known_outputs,
                                                 byte_pointer,
                                                 module.new_string_ptr("known_outputs_ptr"));

    add_function_call(module,
                      bb,
                      "write",
                      &mut vec![stdout_fd, known_outputs_ptr, llvm_num_outputs],
                      "");
}

// TODO: use init_values terminology consistently for names here.
pub fn compile_to_ir(module_name: &str,
                     instrs: &[Instruction],
                     initial_state: &ExecutionState)
                     -> CString {
    let llvm_ir_owned;
    unsafe {
        let mut module = create_module(module_name);

        let main_fn = add_main_fn(&mut module);
        let mut bb = LLVMGetLastBasicBlock(main_fn);

        if initial_state.outputs.len() > 0 {
            compile_static_outputs(&mut module, &mut *bb, &initial_state.outputs);
        }

        if instrs.len() > 0 {
            // TODO: decide on a consistent order between module and bb as
            // parameters.
            let llvm_cells = add_cells_init(&initial_state.cells, &mut module, &mut *bb);
            let llvm_cell_index = add_cell_index_init(initial_state.cell_ptr, bb, &mut module);

            let ctx = CompileContext {
                cells: llvm_cells
            };

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

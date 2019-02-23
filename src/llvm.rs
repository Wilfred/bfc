//! The LLVM module handles converting a BF AST to LLVM IR.

use itertools::Itertools;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;
use llvm_sys::transforms::pass_manager_builder::*;
use llvm_sys::{LLVMBuilder, LLVMIntPredicate, LLVMModule};

use std::ffi::{CStr, CString};
use std::os::raw::{c_uint, c_ulonglong};
use std::ptr::null_mut;
use std::str;

use std::collections::HashMap;
use std::num::Wrapping;

use bfir::AstNode::*;
use bfir::{AstNode, Cell};

use execution::ExecutionState;

const LLVM_FALSE: LLVMBool = 0;
const LLVM_TRUE: LLVMBool = 1;

/// A struct that keeps ownership of all the strings we've passed to
/// the LLVM API until we destroy the `LLVMModule`.
pub struct Module {
    module: *mut LLVMModule,
    strings: Vec<CString>,
}

impl Module {
    /// Create a new CString associated with this LLVMModule,
    /// and return a pointer that can be passed to LLVM APIs.
    /// Assumes s is pure-ASCII.
    fn new_string_ptr(&mut self, s: &str) -> *const i8 {
        self.new_mut_string_ptr(s)
    }

    // TODO: ideally our pointers wouldn't be mutable.
    fn new_mut_string_ptr(&mut self, s: &str) -> *mut i8 {
        let cstring = CString::new(s).unwrap();
        let ptr = cstring.as_ptr() as *mut _;
        self.strings.push(cstring);
        ptr
    }

    pub fn to_cstring(&self) -> CString {
        unsafe {
            // LLVM gives us a *char pointer, so wrap it in a CStr to mark it
            // as borrowed.
            let llvm_ir_ptr = LLVMPrintModuleToString(self.module);
            let llvm_ir = CStr::from_ptr(llvm_ir_ptr as *const _);

            // Make an owned copy of the string in our memory space.
            let module_string = CString::new(llvm_ir.to_bytes()).unwrap();

            // Cleanup borrowed string.
            LLVMDisposeMessage(llvm_ir_ptr);

            module_string
        }
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        // Rust requires that drop() is a safe function.
        unsafe {
            LLVMDisposeModule(self.module);
        }
    }
}

/// Wraps LLVM's builder class to provide a nicer API and ensure we
/// always dispose correctly.
struct Builder {
    builder: *mut LLVMBuilder,
}

impl Builder {
    /// Create a new Builder in LLVM's global context.
    fn new() -> Self {
        unsafe {
            Builder {
                builder: LLVMCreateBuilder(),
            }
        }
    }

    fn position_at_end(&self, bb: LLVMBasicBlockRef) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.builder, bb);
        }
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
    cells: LLVMValueRef,
    cell_index_ptr: LLVMValueRef,
    main_fn: LLVMValueRef,
}

/// Convert this integer to LLVM's representation of a constant
/// integer.
unsafe fn int8(val: c_ulonglong) -> LLVMValueRef {
    LLVMConstInt(LLVMInt8Type(), val, LLVM_FALSE)
}
/// Convert this integer to LLVM's representation of a constant
/// integer.
// TODO: this should be a machine word size rather than hard-coding 32-bits.
fn int32(val: c_ulonglong) -> LLVMValueRef {
    unsafe { LLVMConstInt(LLVMInt32Type(), val, LLVM_FALSE) }
}

fn int1_type() -> LLVMTypeRef {
    unsafe { LLVMInt1Type() }
}

fn int8_type() -> LLVMTypeRef {
    unsafe { LLVMInt8Type() }
}

fn int32_type() -> LLVMTypeRef {
    unsafe { LLVMInt32Type() }
}

fn int8_ptr_type() -> LLVMTypeRef {
    unsafe { LLVMPointerType(LLVMInt8Type(), 0) }
}

fn add_function(
    module: &mut Module,
    fn_name: &str,
    args: &mut [LLVMTypeRef],
    ret_type: LLVMTypeRef,
) {
    unsafe {
        let fn_type = LLVMFunctionType(ret_type, args.as_mut_ptr(), args.len() as u32, LLVM_FALSE);
        LLVMAddFunction(module.module, module.new_string_ptr(fn_name), fn_type);
    }
}

fn add_c_declarations(module: &mut Module) {
    let void;
    unsafe {
        void = LLVMVoidType();
    }

    add_function(
        module,
        "llvm.memset.p0i8.i32",
        &mut [
            int8_ptr_type(),
            int8_type(),
            int32_type(),
            int32_type(),
            int1_type(),
        ],
        void,
    );

    add_function(module, "malloc", &mut [int32_type()], int8_ptr_type());

    add_function(module, "free", &mut [int8_ptr_type()], void);

    add_function(
        module,
        "write",
        &mut [int32_type(), int8_ptr_type(), int32_type()],
        int32_type(),
    );

    add_function(module, "putchar", &mut [int32_type()], int32_type());

    add_function(module, "getchar", &mut [], int32_type());
}

unsafe fn add_function_call(
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    fn_name: &str,
    args: &mut [LLVMValueRef],
    name: &str,
) -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let function = LLVMGetNamedFunction(module.module, module.new_string_ptr(fn_name));

    LLVMBuildCall(
        builder.builder,
        function,
        args.as_mut_ptr(),
        args.len() as c_uint,
        module.new_string_ptr(name),
    )
}

/// Given a vector of cells [1, 1, 0, 0, 0, ...] return a vector
/// [(1, 2), (0, 3), ...].
fn run_length_encode<T>(cells: &[T]) -> Vec<(T, usize)>
where
    T: Eq + Copy,
{
    cells
        .iter()
        .map(|val| (*val, 1))
        .coalesce(|(prev_val, prev_count), (val, count)| {
            if prev_val == val {
                Ok((val, prev_count + count))
            } else {
                Err(((prev_val, prev_count), (val, count)))
            }
        })
        .collect()
}

fn add_cells_init(
    init_values: &[Wrapping<i8>],
    module: &mut Module,
    bb: LLVMBasicBlockRef,
) -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    unsafe {
        // char* cells = malloc(num_cells);
        let num_cells = int32(init_values.len() as c_ulonglong);
        let mut malloc_args = vec![num_cells];
        let cells_ptr = add_function_call(module, bb, "malloc", &mut malloc_args, "cells");

        let one = int32(1);
        let false_ = LLVMConstInt(int1_type(), 1, LLVM_FALSE);

        let mut offset = 0;
        for (cell_val, cell_count) in run_length_encode(init_values) {
            let llvm_cell_val = int8(cell_val.0 as c_ulonglong);
            let llvm_cell_count = int32(cell_count as c_ulonglong);

            // TODO: factor out a build_gep function.
            let mut offset_vec = vec![int32(offset as c_ulonglong)];
            let offset_cell_ptr = LLVMBuildGEP(
                builder.builder,
                cells_ptr,
                offset_vec.as_mut_ptr(),
                offset_vec.len() as u32,
                module.new_string_ptr("offset_cell_ptr"),
            );

            let mut memset_args =
                vec![offset_cell_ptr, llvm_cell_val, llvm_cell_count, one, false_];
            add_function_call(module, bb, "llvm.memset.p0i8.i32", &mut memset_args, "");

            offset += cell_count;
        }

        cells_ptr
    }
}

fn add_cells_cleanup(module: &mut Module, bb: LLVMBasicBlockRef, cells: LLVMValueRef) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    unsafe {
        // free(cells);
        let mut free_args = vec![cells];
        add_function_call(module, bb, "free", &mut free_args, "");
    }
}

fn create_module(module_name: &str, target_triple: Option<String>) -> Module {
    let c_module_name = CString::new(module_name).unwrap();
    let module_name_char_ptr = c_module_name.to_bytes_with_nul().as_ptr() as *const _;

    let llvm_module;
    unsafe {
        llvm_module = LLVMModuleCreateWithName(module_name_char_ptr);
    }
    let mut module = Module {
        module: llvm_module,
        strings: vec![c_module_name],
    };

    let target_triple_cstring = if let Some(target_triple) = target_triple {
        CString::new(target_triple).unwrap()
    } else {
        get_default_target_triple()
    };

    // This is necessary for maximum LLVM performance, see
    // http://llvm.org/docs/Frontend/PerformanceTips.html
    unsafe {
        LLVMSetTarget(llvm_module, target_triple_cstring.as_ptr() as *const _);
    }
    // TODO: add a function to the LLVM C API that gives us the
    // data layout from the target machine.

    add_c_declarations(&mut module);
    module
}

fn add_main_fn(module: &mut Module) -> LLVMValueRef {
    let mut main_args = vec![];
    unsafe {
        let main_type = LLVMFunctionType(int32_type(), main_args.as_mut_ptr(), 0, LLVM_FALSE);
        // TODO: use add_function() here instead.
        LLVMAddFunction(module.module, module.new_string_ptr("main"), main_type)
    }
}

/// Set up the initial basic blocks for appending instructions.
fn add_initial_bbs(
    module: &mut Module,
    main_fn: LLVMValueRef,
) -> (LLVMBasicBlockRef, LLVMBasicBlockRef) {
    unsafe {
        // This basic block is empty, but we will add a branch during
        // compilation according to InstrPosition.
        let init_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("init"));

        // We'll begin by appending instructions here.
        let beginning_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("beginning"));

        (init_bb, beginning_bb)
    }
}

// TODO: name our pointers cell_base and
// cell_offset_ptr.
/// Initialise the value that contains the current cell index.
unsafe fn add_cell_index_init(
    init_value: isize,
    bb: LLVMBasicBlockRef,
    module: &mut Module,
) -> LLVMValueRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    // int cell_index = 0;
    let cell_index_ptr = LLVMBuildAlloca(
        builder.builder,
        int32_type(),
        module.new_string_ptr("cell_index_ptr"),
    );
    let cell_ptr_init = int32(init_value as c_ulonglong);
    LLVMBuildStore(builder.builder, cell_ptr_init, cell_index_ptr);

    cell_index_ptr
}

/// Add prologue to main function.
unsafe fn add_main_cleanup(bb: LLVMBasicBlockRef) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let zero = int32(0);
    LLVMBuildRet(builder.builder, zero);
}

/// Add LLVM IR instructions for accessing the current cell, and
/// return a reference to the current cell, and to a current cell pointer.
unsafe fn add_current_cell_access(
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    cells: LLVMValueRef,
    cell_index_ptr: LLVMValueRef,
) -> (LLVMValueRef, LLVMValueRef) {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(
        builder.builder,
        cell_index_ptr,
        module.new_string_ptr("cell_index"),
    );

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder.builder,
        cells,
        indices.as_mut_ptr(),
        indices.len() as u32,
        module.new_string_ptr("current_cell_ptr"),
    );
    let current_cell = LLVMBuildLoad(
        builder.builder,
        current_cell_ptr,
        module.new_string_ptr("cell_value"),
    );

    (current_cell, current_cell_ptr)
}

unsafe fn compile_increment(
    amount: Cell,
    offset: isize,
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(
        builder.builder,
        ctx.cell_index_ptr,
        module.new_string_ptr("cell_index"),
    );

    let offset_cell_index = LLVMBuildAdd(
        builder.builder,
        cell_index,
        int32(offset as c_ulonglong),
        module.new_string_ptr("offset_cell_index"),
    );

    let mut indices = vec![offset_cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder.builder,
        ctx.cells,
        indices.as_mut_ptr(),
        indices.len() as c_uint,
        module.new_string_ptr("current_cell_ptr"),
    );

    let cell_val = LLVMBuildLoad(
        builder.builder,
        current_cell_ptr,
        module.new_string_ptr("cell_value"),
    );

    let increment_amount = int8(amount.0 as c_ulonglong);
    let new_cell_val = LLVMBuildAdd(
        builder.builder,
        cell_val,
        increment_amount,
        module.new_string_ptr("new_cell_value"),
    );

    LLVMBuildStore(builder.builder, new_cell_val, current_cell_ptr);
    bb
}

unsafe fn compile_set(
    amount: Cell,
    offset: isize,
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(
        builder.builder,
        ctx.cell_index_ptr,
        module.new_string_ptr("cell_index"),
    );

    let offset_cell_index = LLVMBuildAdd(
        builder.builder,
        cell_index,
        int32(offset as c_ulonglong),
        module.new_string_ptr("offset_cell_index"),
    );

    let mut indices = vec![offset_cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder.builder,
        ctx.cells,
        indices.as_mut_ptr(),
        indices.len() as c_uint,
        module.new_string_ptr("current_cell_ptr"),
    );

    LLVMBuildStore(
        builder.builder,
        int8(amount.0 as c_ulonglong),
        current_cell_ptr,
    );
    bb
}

unsafe fn compile_multiply_move(
    changes: &HashMap<isize, Cell>,
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let multiply_body = LLVMAppendBasicBlock(ctx.main_fn, module.new_string_ptr("multiply_body"));
    let multiply_after = LLVMAppendBasicBlock(ctx.main_fn, module.new_string_ptr("multiply_after"));

    let builder = Builder::new();
    builder.position_at_end(bb);

    // First, get the current cell value.
    let (cell_val, cell_val_ptr) =
        add_current_cell_access(module, bb, ctx.cells, ctx.cell_index_ptr);

    // Check if the current cell is zero, as we only do the multiply
    // if it's non-zero.
    let zero = int8(0);
    let cell_val_is_zero = LLVMBuildICmp(
        builder.builder,
        LLVMIntPredicate::LLVMIntEQ,
        zero,
        cell_val,
        module.new_string_ptr("cell_value_is_zero"),
    );
    LLVMBuildCondBr(
        builder.builder,
        cell_val_is_zero,
        multiply_after,
        multiply_body,
    );

    // In the multiply body, do the mulitply
    builder.position_at_end(multiply_body);

    // Zero the current cell.
    LLVMBuildStore(builder.builder, int8(0), cell_val_ptr);

    let mut targets: Vec<_> = changes.keys().collect();
    targets.sort();

    // For each cell that we should change, multiply the current cell
    // value then add it.
    for target in targets {
        // Calculate the position of this target cell.
        let mut indices = vec![int32(*target as c_ulonglong)];
        let target_cell_ptr = LLVMBuildGEP(
            builder.builder,
            cell_val_ptr,
            indices.as_mut_ptr(),
            indices.len() as c_uint,
            module.new_string_ptr("target_cell_ptr"),
        );

        // Get the current value of the target cell.
        let target_cell_val = LLVMBuildLoad(
            builder.builder,
            target_cell_ptr,
            module.new_string_ptr("target_cell_val"),
        );

        // Calculate the new value.
        let factor = *changes.get(target).unwrap();
        let additional_val = LLVMBuildMul(
            builder.builder,
            cell_val,
            int8(factor.0 as c_ulonglong),
            module.new_string_ptr("additional_val"),
        );
        let new_target_val = LLVMBuildAdd(
            builder.builder,
            target_cell_val,
            additional_val,
            module.new_string_ptr("new_target_val"),
        );
        LLVMBuildStore(builder.builder, new_target_val, target_cell_ptr);
    }

    // Finally, continue execution from multiply after.
    LLVMBuildBr(builder.builder, multiply_after);

    multiply_after
}

unsafe fn compile_ptr_increment(
    amount: isize,
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(
        builder.builder,
        ctx.cell_index_ptr,
        module.new_string_ptr("cell_index"),
    );

    let new_cell_index = LLVMBuildAdd(
        builder.builder,
        cell_index,
        int32(amount as c_ulonglong),
        module.new_string_ptr("new_cell_index"),
    );

    LLVMBuildStore(builder.builder, new_cell_index, ctx.cell_index_ptr);
    bb
}

unsafe fn compile_read(
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_index = LLVMBuildLoad(
        builder.builder,
        ctx.cell_index_ptr,
        module.new_string_ptr("cell_index"),
    );

    let mut indices = vec![cell_index];
    let current_cell_ptr = LLVMBuildGEP(
        builder.builder,
        ctx.cells,
        indices.as_mut_ptr(),
        indices.len() as u32,
        module.new_string_ptr("current_cell_ptr"),
    );

    let mut getchar_args = vec![];
    let input_char = add_function_call(module, bb, "getchar", &mut getchar_args, "input_char");
    let input_byte = LLVMBuildTrunc(
        builder.builder,
        input_char,
        int8_type(),
        module.new_string_ptr("input_byte"),
    );

    LLVMBuildStore(builder.builder, input_byte, current_cell_ptr);
    bb
}

unsafe fn compile_write(
    module: &mut Module,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let builder = Builder::new();
    builder.position_at_end(bb);

    let cell_val = add_current_cell_access(module, bb, ctx.cells, ctx.cell_index_ptr).0;
    let cell_val_as_char = LLVMBuildSExt(
        builder.builder,
        cell_val,
        int32_type(),
        module.new_string_ptr("cell_val_as_char"),
    );

    let mut putchar_args = vec![cell_val_as_char];
    add_function_call(module, bb, "putchar", &mut putchar_args, "");
    bb
}

fn ptr_equal<T>(a: *const T, b: *const T) -> bool {
    a == b
}

unsafe fn compile_loop(
    loop_body: &[AstNode],
    start_instr: &AstNode,
    module: &mut Module,
    main_fn: LLVMValueRef,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    let builder = Builder::new();

    // First, we branch into the loop header from the previous basic
    // block.
    let loop_header_bb = LLVMAppendBasicBlock(ctx.main_fn, module.new_string_ptr("loop_header"));
    builder.position_at_end(bb);
    LLVMBuildBr(builder.builder, loop_header_bb);

    let mut loop_body_bb = LLVMAppendBasicBlock(ctx.main_fn, module.new_string_ptr("loop_body"));
    let loop_after = LLVMAppendBasicBlock(ctx.main_fn, module.new_string_ptr("loop_after"));

    // loop_header:
    //   %cell_value = ...
    //   %cell_value_is_zero = icmp ...
    //   br %cell_value_is_zero, %loop_after, %loop_body
    builder.position_at_end(loop_header_bb);

    let cell_val =
        add_current_cell_access(module, &mut *loop_header_bb, ctx.cells, ctx.cell_index_ptr).0;

    let zero = int8(0);
    let cell_val_is_zero = LLVMBuildICmp(
        builder.builder,
        LLVMIntPredicate::LLVMIntEQ,
        zero,
        cell_val,
        module.new_string_ptr("cell_value_is_zero"),
    );
    LLVMBuildCondBr(builder.builder, cell_val_is_zero, loop_after, loop_body_bb);

    // Recursively compile instructions in the loop body.
    for instr in loop_body {
        if ptr_equal(instr, start_instr) {
            // This is the point we want to start execution from.
            loop_body_bb = set_entry_point_after(module, main_fn, loop_body_bb);
        }

        loop_body_bb = compile_instr(
            instr,
            start_instr,
            module,
            main_fn,
            loop_body_bb,
            ctx.clone(),
        );
    }

    // When the loop is finished, jump back to the beginning of the
    // loop.
    builder.position_at_end(loop_body_bb);
    LLVMBuildBr(builder.builder, loop_header_bb);

    &mut *loop_after
}

/// Append LLVM IR instructions to bb acording to the BF instruction
/// passed in.
unsafe fn compile_instr(
    instr: &AstNode,
    start_instr: &AstNode,
    module: &mut Module,
    main_fn: LLVMValueRef,
    bb: LLVMBasicBlockRef,
    ctx: CompileContext,
) -> LLVMBasicBlockRef {
    match *instr {
        Increment { amount, offset, .. } => compile_increment(amount, offset, module, bb, ctx),
        Set { amount, offset, .. } => compile_set(amount, offset, module, bb, ctx),
        MultiplyMove { ref changes, .. } => compile_multiply_move(changes, module, bb, ctx),
        PointerIncrement { amount, .. } => compile_ptr_increment(amount, module, bb, ctx),
        Read { .. } => compile_read(module, bb, ctx),
        Write { .. } => compile_write(module, bb, ctx),
        Loop { ref body, .. } => compile_loop(body, start_instr, module, main_fn, bb, ctx),
    }
}

fn compile_static_outputs(module: &mut Module, bb: LLVMBasicBlockRef, outputs: &[i8]) {
    unsafe {
        let builder = Builder::new();
        builder.position_at_end(bb);

        let mut llvm_outputs = vec![];
        for value in outputs {
            llvm_outputs.push(int8(*value as c_ulonglong));
        }

        let output_buf_type = LLVMArrayType(int8_type(), llvm_outputs.len() as c_uint);
        let llvm_outputs_arr = LLVMConstArray(
            int8_type(),
            llvm_outputs.as_mut_ptr(),
            llvm_outputs.len() as c_uint,
        );

        let known_outputs = LLVMAddGlobal(
            module.module,
            output_buf_type,
            module.new_string_ptr("known_outputs"),
        );
        LLVMSetInitializer(known_outputs, llvm_outputs_arr);
        LLVMSetGlobalConstant(known_outputs, LLVM_TRUE);

        let stdout_fd = int32(1);
        let llvm_num_outputs = int32(outputs.len() as c_ulonglong);

        let known_outputs_ptr = LLVMBuildPointerCast(
            builder.builder,
            known_outputs,
            int8_ptr_type(),
            module.new_string_ptr("known_outputs_ptr"),
        );

        add_function_call(
            module,
            bb,
            "write",
            &mut [stdout_fd, known_outputs_ptr, llvm_num_outputs],
            "",
        );
    }
}

/// Ensure that execution starts after the basic block we pass in.
unsafe fn set_entry_point_after(
    module: &mut Module,
    main_fn: LLVMValueRef,
    bb: LLVMBasicBlockRef,
) -> LLVMBasicBlockRef {
    let after_init_bb = LLVMAppendBasicBlock(main_fn, module.new_string_ptr("after_init"));

    // From the current bb, we want to continue execution in after_init.
    let builder = Builder::new();
    builder.position_at_end(bb);
    LLVMBuildBr(builder.builder, after_init_bb);

    // We also want to start execution in after_init.
    let init_bb = LLVMGetFirstBasicBlock(main_fn);
    builder.position_at_end(init_bb);
    LLVMBuildBr(builder.builder, after_init_bb);

    after_init_bb
}

// TODO: use init_values terminology consistently for names here.
pub fn compile_to_module(
    module_name: &str,
    target_triple: Option<String>,
    instrs: &[AstNode],
    initial_state: &ExecutionState,
) -> Module {
    let mut module = create_module(module_name, target_triple);
    let main_fn = add_main_fn(&mut module);

    let (init_bb, mut bb) = add_initial_bbs(&mut module, main_fn);

    if !initial_state.outputs.is_empty() {
        compile_static_outputs(&mut module, init_bb, &initial_state.outputs);
    }

    unsafe {
        // If there's no start instruction, then we executed all
        // instructions at compile time and we don't need to do anything here.
        match initial_state.start_instr {
            Some(start_instr) => {
                // TODO: decide on a consistent order between module and init_bb as
                // parameters.
                let llvm_cells = add_cells_init(&initial_state.cells, &mut module, init_bb);
                let llvm_cell_index =
                    add_cell_index_init(initial_state.cell_ptr, init_bb, &mut module);

                let ctx = CompileContext {
                    cells: llvm_cells,
                    cell_index_ptr: llvm_cell_index,
                    main_fn,
                };

                for instr in instrs {
                    if ptr_equal(instr, start_instr) {
                        // This is the point we want to start execution from.
                        bb = set_entry_point_after(&mut module, main_fn, bb);
                    }

                    bb = compile_instr(instr, start_instr, &mut module, main_fn, bb, ctx.clone());
                }

                add_cells_cleanup(&mut module, bb, llvm_cells);
            }
            None => {
                // We won't have called set_entry_point_after, so set
                // the entry point.
                let builder = Builder::new();
                builder.position_at_end(init_bb);
                LLVMBuildBr(builder.builder, bb);
            }
        }

        add_main_cleanup(bb);

        module
    }
}

pub fn optimise_ir(module: &mut Module, llvm_opt: i64) {
    // TODO: add a verifier pass too.
    unsafe {
        let builder = LLVMPassManagerBuilderCreate();
        // E.g. if llvm_opt is 3, we want a pass equivalent to -O3.
        LLVMPassManagerBuilderSetOptLevel(builder, llvm_opt as u32);

        let pass_manager = LLVMCreatePassManager();
        LLVMPassManagerBuilderPopulateModulePassManager(builder, pass_manager);

        LLVMPassManagerBuilderDispose(builder);

        // Run twice. This is a hack, we should really work out which
        // optimisations need to run twice. See
        // http://llvm.org/docs/Frontend/PerformanceTips.html#pass-ordering
        LLVMRunPassManager(pass_manager, module.module);
        LLVMRunPassManager(pass_manager, module.module);

        LLVMDisposePassManager(pass_manager);
    }
}

pub fn get_default_target_triple() -> CString {
    let target_triple;
    unsafe {
        let target_triple_ptr = LLVMGetDefaultTargetTriple();
        target_triple = CStr::from_ptr(target_triple_ptr as *const _).to_owned();
        LLVMDisposeMessage(target_triple_ptr);
    }

    target_triple
}

struct TargetMachine {
    tm: LLVMTargetMachineRef,
}

impl TargetMachine {
    fn new(target_triple: *const i8) -> Result<Self, String> {
        let mut target = null_mut();
        let mut err_msg_ptr = null_mut();
        unsafe {
            LLVMGetTargetFromTriple(target_triple, &mut target, &mut err_msg_ptr);
            if target.is_null() {
                // LLVM couldn't find a target triple with this name,
                // so it should have given us an error message.
                assert!(!err_msg_ptr.is_null());

                let err_msg_cstr = CStr::from_ptr(err_msg_ptr as *const _);
                let err_msg = str::from_utf8(err_msg_cstr.to_bytes()).unwrap();
                return Err(err_msg.to_owned());
            }
        }

        // TODO: do these strings live long enough?
        // cpu is documented: http://llvm.org/docs/CommandGuide/llc.html#cmdoption-mcpu
        let cpu = CString::new("generic").unwrap();
        // features are documented: http://llvm.org/docs/CommandGuide/llc.html#cmdoption-mattr
        let features = CString::new("").unwrap();

        let target_machine;
        unsafe {
            target_machine = LLVMCreateTargetMachine(
                target,
                target_triple,
                cpu.as_ptr() as *const _,
                features.as_ptr() as *const _,
                LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
                LLVMRelocMode::LLVMRelocPIC,
                LLVMCodeModel::LLVMCodeModelDefault,
            );
        }

        Ok(TargetMachine { tm: target_machine })
    }
}

impl Drop for TargetMachine {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeTargetMachine(self.tm);
        }
    }
}

pub fn write_object_file(module: &mut Module, path: &str) -> Result<(), String> {
    unsafe {
        let target_triple = LLVMGetTarget(module.module);

        // TODO: are all these necessary? Are there docs?
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmParsers();
        LLVM_InitializeAllAsmPrinters();

        let target_machine = try!(TargetMachine::new(target_triple));

        let mut obj_error = module.new_mut_string_ptr("Writing object file failed.");
        let result = LLVMTargetMachineEmitToFile(
            target_machine.tm,
            module.module,
            module.new_string_ptr(path) as *mut i8,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut obj_error,
        );

        if result != 0 {
            println!("obj_error: {:?}", CStr::from_ptr(obj_error as *const _));
            assert!(false);
        }
    }
    Ok(())
}

extern crate llvm_sys as llvm;

use llvm::core::*;

mod bfir;

unsafe fn add_c_declarations(module: &mut llvm::LLVMModule) {
    let byte_pointer = LLVMPointerType(
        LLVMInt8Type(), 0);
    
    let mut calloc_args = vec![LLVMInt32Type(), LLVMInt32Type()];
    let calloc_type = 
        LLVMFunctionType(byte_pointer, calloc_args.as_mut_ptr(), 2, 0);
    LLVMAddFunction(module, b"calloc\0".as_ptr() as *const _, calloc_type);

    let mut free_args = vec![byte_pointer];
    let free_type = LLVMFunctionType(
        LLVMVoidType(), free_args.as_mut_ptr(), 1, 0);
    LLVMAddFunction(module, b"free\0".as_ptr() as *const _, free_type);
}

unsafe fn emit_llvm_ir() {
    // Set up a context, module and builder in that context.
    let context = LLVMGetGlobalContext();
    let module = LLVMModuleCreateWithName(b"nop\0".as_ptr() as *const _);
    let builder = LLVMCreateBuilderInContext(context);

    add_c_declarations(&mut *module);

    // Dump the module as IR to stdout.
    LLVMDumpModule(module);

    // Clean up. Values created in the context mostly get cleaned up there.
    LLVMDisposeBuilder(builder);
    LLVMDisposeModule(module);
}

fn main() {
    unsafe {
        emit_llvm_ir();
    }
}

extern crate llvm_sys as llvm;

use std::ptr;

mod bfir;

unsafe fn add_c_declarations(module: &mut llvm::LLVMModule) {
    let byte_pointer = llvm::core::LLVMPointerType(
        llvm::core::LLVMInt8Type(), 0);
    
    let mut calloc_args = vec![llvm::core::LLVMInt32Type(),
                           llvm::core::LLVMInt32Type()];
    let calloc_type =
        llvm::core::LLVMFunctionType(
            byte_pointer, calloc_args.as_mut_ptr(), 2, 0);
    llvm::core::LLVMAddFunction(module, b"calloc\0".as_ptr() as *const _,
                                calloc_type);

    let mut free_args = vec![byte_pointer];
    let free_type = llvm::core::LLVMFunctionType(
        llvm::core::LLVMVoidType(), free_args.as_mut_ptr(), 1, 0);
    llvm::core::LLVMAddFunction(module, b"free\0".as_ptr() as *const _, free_type);
}

unsafe fn emit_llvm_ir() {
    // Set up a context, module and builder in that context.
    let context = llvm::core::LLVMGetGlobalContext();
    let module = llvm::core::LLVMModuleCreateWithName(b"nop\0".as_ptr() as *const _);
    let builder = llvm::core::LLVMCreateBuilderInContext(context);

    add_c_declarations(&mut *module);

    // Dump the module as IR to stdout.
    llvm::core::LLVMDumpModule(module);

    // Clean up. Values created in the context mostly get cleaned up there.
    llvm::core::LLVMDisposeBuilder(builder);
    llvm::core::LLVMDisposeModule(module);
}

fn main() {
    unsafe {
        emit_llvm_ir();
    }
}

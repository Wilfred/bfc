extern crate llvm_sys as llvm;

use std::env;
use std::fs::File;
use std::io::prelude::Read;

use llvm::core::*;

mod bfir;

unsafe fn add_c_declarations(module: &mut llvm::LLVMModule) {
    let byte_pointer = LLVMPointerType(LLVMInt8Type(), 0);
    
    let mut calloc_args = vec![LLVMInt32Type(), LLVMInt32Type()];
    let calloc_type = 
        LLVMFunctionType(byte_pointer, calloc_args.as_mut_ptr(), 2, 0);
    LLVMAddFunction(module, b"calloc\0".as_ptr() as *const _, calloc_type);

    let mut free_args = vec![byte_pointer];
    let free_type = LLVMFunctionType(
        LLVMVoidType(), free_args.as_mut_ptr(), 1, 0);
    LLVMAddFunction(module, b"free\0".as_ptr() as *const _, free_type);

    let mut putchar_args = vec![LLVMInt32Type()];
    let putchar_type = LLVMFunctionType(
        LLVMInt32Type(), putchar_args.as_mut_ptr(), 1, 0);
    LLVMAddFunction(module, b"putchar\0".as_ptr() as *const _, putchar_type);

    let mut getchar_args = vec![];
    let getchar_type = LLVMFunctionType(
        LLVMInt32Type(), getchar_args.as_mut_ptr(), 0, 0);
    LLVMAddFunction(module, b"getchar\0".as_ptr() as *const _, getchar_type);
}

unsafe fn emit_llvm_ir() {
    let context = LLVMGetGlobalContext();
    let module = LLVMModuleCreateWithName(b"nop\0".as_ptr() as *const _);
    let builder = LLVMCreateBuilderInContext(context);

    add_c_declarations(&mut *module);

    let mut main_args = vec![];
    let main_type = LLVMFunctionType(
        LLVMInt32Type(), main_args.as_mut_ptr(), 0, 0);
    let main_fn = LLVMAddFunction(module, b"main\0".as_ptr() as *const _,
                                  main_type);

    let bb = LLVMAppendBasicBlockInContext(
        context, main_fn, b"entry\0".as_ptr() as *const _);
    LLVMPositionBuilderAtEnd(builder, bb);
    LLVMBuildRetVoid(builder);

    // Dump the module as IR to stdout.
    LLVMDumpModule(module);

    LLVMDisposeBuilder(builder);
    LLVMDisposeModule(module);
}

/// Read the contents of the file at path, and return a string of its
/// contents.
fn slurp(path: &str) -> Result<String, std::io::Error> {
    let mut file = try!(File::open(path));
    let mut contents = String::new();
    try!(file.read_to_string(&mut contents));
    Ok(contents)
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() == 2 {
        let ref file_name = args[1];
        match slurp(&file_name) {
            Ok(src) => {
                println!("src: {}", src);
            }
            Err(e) => {
                println!("Could not open file: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("You need to provide a file to compile.");
        println!("For example: {} foo.bf", args[0]);
        std::process::exit(1);
    }
    
    unsafe {
        emit_llvm_ir();
    }
}

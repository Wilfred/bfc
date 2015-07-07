#![feature(cstr_memory)]

extern crate llvm_sys;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::Read;
use std::path::Path;

mod bfir;
mod llvm;
mod optimize;

/// Read the contents of the file at path, and return a string of its
/// contents.
fn slurp(path: &str) -> Result<String, std::io::Error> {
    let mut file = try!(File::open(path));
    let mut contents = String::new();
    try!(file.read_to_string(&mut contents));
    Ok(contents)
}

/// Convert "foo.bf" to "foo.ll".
fn ll_file_name(bf_file_name: &str) -> String {
    let mut name_parts: Vec<_> = bf_file_name.split('.').collect();
    let parts_len = name_parts.len();
    if parts_len > 1 {
        name_parts[parts_len - 1] = "ll";
    } else {
        name_parts.push("ll");
    }

    name_parts.connect(".")
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {

        // TODO: proper options parsing
        let dump_bf_ir = args.len() > 2 && args[2] == "--dump-bf-ir";
        let dump_llvm = args.len() > 2 && args[2] == "--dump-llvm";
        
        let ref file_path = args[1];
        match slurp(&file_path) {
            Ok(src) => {
                let instrs = bfir::parse(&src);
                if dump_bf_ir {
                    for instr in &instrs {
                        println!("{}", instr);
                    }
                    return
                }

                unsafe {
                    let llvm_ir_raw = llvm::compile_to_ir(&file_path, &instrs);

                    if dump_llvm {
                        let llvm_ir = String::from_utf8_lossy(llvm_ir_raw.as_bytes());
                        println!("{}", llvm_ir);
                    } else {
                        // TODO: write to a temp file then call llc.
                        let bf_name = Path::new(file_path).file_name().unwrap();
                        let ll_name = ll_file_name(bf_name.to_str().unwrap());
                        match File::create(&ll_name) {
                            Ok(mut f) => {
                                let _ = f.write(llvm_ir_raw.as_bytes());
                                println!("Wrote {}", ll_name);
                            }
                            Err(e) => {
                                println!("Could not create file: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }

            }
            Err(e) => {
                println!("Could not open file: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("Usage: {} <BF source file> [options...]", args[0]);
        println!("Examples:");
        println!("  {} foo.bf", args[0]);
        println!("  {} foo.bf --dump-bf-ir", args[0]);
        println!("  {} foo.bf --dump-llvm", args[0]);
        std::process::exit(1);
    }
    
}

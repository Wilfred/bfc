extern crate llvm_sys;

use std::env;
use std::fs::File;
use std::io::prelude::Read;

mod bfir;
mod llvm;

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
                let instrs = bfir::parse(&src);
                for instr in instrs {
                    println!("{}", instr);
                }

                unsafe {
                    llvm::dump_ir(&file_name);
                }

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
    
}

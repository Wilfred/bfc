#![feature(plugin)]
#![plugin(quickcheck_macros)]

// TODO: find a way to avoid this.
#![feature(convert)]

extern crate libc;
extern crate llvm_sys;
extern crate itertools;
extern crate quickcheck;
extern crate rand;
extern crate tempfile;

use std::env;
use std::fmt::{Display,Formatter};
use std::fs::File;
use std::io::Write;
use std::io::prelude::Read;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

mod bfir;
mod llvm;
mod optimize;
mod bounds;
mod execution;

#[cfg(test)]
mod optimize_tests;
#[cfg(test)]
mod llvm_tests;

/// Read the contents of the file at path, and return a string of its
/// contents.
fn slurp(path: &str) -> Result<String, std::io::Error> {
    let mut file = try!(File::open(path));
    let mut contents = String::new();
    try!(file.read_to_string(&mut contents));
    Ok(contents)
}

/// Convert "foo.bf" to "foo.o".
#[allow(deprecated)] // .connect is in stable 1.2, but beta has deprecated it.
fn obj_file_name(bf_file_name: &str) -> String {
    let mut name_parts: Vec<_> = bf_file_name.split('.').collect();
    let parts_len = name_parts.len();
    if parts_len > 1 {
        name_parts[parts_len - 1] = "o";
    } else {
        name_parts.push("o");
    }

    name_parts.connect(".")
}

fn print_usage(bin_name: &str) {
    println!("Usage: {} <BF source file> [options...]", bin_name);
    println!("Examples:");
    println!("  {} foo.bf", bin_name);
    println!("  {} foo.bf --dump-bf-ir", bin_name);
    println!("  {} foo.bf --dump-llvm", bin_name);
}

#[derive(Debug)]
struct StringError { message: String }

impl StringError {
    fn new(message: &str) -> StringError {
        StringError { message: message.to_owned() }
    }
}

impl std::error::Error for StringError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl Display for StringError {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), std::fmt::Error> {
        self.message.fmt(formatter)
    }
}

fn compile_file(path: &str, dump_bf_ir: bool, dump_llvm: bool)
                -> Result<(),std::io::Error> {
    let src = try!(slurp(&path));

    // TODO: wrapping everything in io::Error is ugly.
    let instrs;
    match bfir::parse(&src) {
        Ok(instrs_) => {
            instrs = instrs_;
        }
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other, StringError::new(&e)));
        }
    }
    
    // TODO: allow users to specify optimisation level.
    let instrs = optimize::optimize(instrs);

    if dump_bf_ir {
        for instr in &instrs {
            println!("{}", instr);
        }
        return Ok(());
    }

    // TODO: highest_cell_index should return a usize.
    let state = execution::execute(&instrs, execution::MAX_STEPS);

    let remaining_instrs = &instrs[state.instr_ptr..];
    let llvm_ir_raw = llvm::compile_to_ir(
        &path, &remaining_instrs.to_vec(), &state.cells, state.cell_ptr as i32,
        &state.outputs);

    if dump_llvm {
        let llvm_ir = String::from_utf8_lossy(llvm_ir_raw.as_bytes());
        println!("{}", llvm_ir);
        return Ok(());
    }                        

    let bf_name = Path::new(path).file_name().unwrap();
    let obj_name = obj_file_name(bf_name.to_str().unwrap());

    let mut f = try!(NamedTempFile::new());

    let _ = f.write(llvm_ir_raw.as_bytes());

    // TODO: link as well.
    let llc_result = try!(
        Command::new("llc")
            .arg("-O3").arg("-filetype=obj").arg(f.path())
            .arg("-o").arg(obj_name.to_owned())
            .output());

    let mut llc_stderr = String::from_utf8_lossy(llc_result.stderr.as_slice());

    // TODO: it would be nice to have a function that wraps
    // Command::new so we don't have to inspect .status.success() and
    // wrap in Err as below.
    if llc_result.status.success() {
        // TODO: it would be cleaner to return this in Ok().
        println!("Wrote {}", obj_name);
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other, StringError::new(llc_stderr.to_mut())))
    }
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {

        // TODO: proper options parsing
        let dump_bf_ir = args.len() > 2 && args[2] == "--dump-bf-ir";
        let dump_llvm = args.len() > 2 && args[2] == "--dump-llvm";
        
        let ref file_path = args[1];

        match compile_file(file_path, dump_bf_ir, dump_llvm) {
            Ok(_) => {}
            Err(e) => {
                // TODO: this should go to stderr.
                println!("{}", e);
                std::process::exit(2);
            }
        }
        
    } else {
        print_usage(&args[0]);
        std::process::exit(1);
    }
}

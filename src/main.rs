#![feature(plugin)]
#![plugin(quickcheck_macros)]

#![warn(trivial_numeric_casts)]

// TODO: find a way to avoid this.
#![feature(convert)]

extern crate libc;
extern crate llvm_sys;
extern crate itertools;
extern crate quickcheck;
extern crate rand;
extern crate tempfile;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::Read;
use std::num::Wrapping;
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

fn convert_io_error<T>(result: Result<T, std::io::Error>) -> Result<T, String> {
    match result {
        Ok(value) => {
            Ok(value)
        }
        Err(e) => {
            Err(format!("{}", e))
        }
    }
}

fn compile_file(path: &str, dump_bf_ir: bool, dump_llvm: bool)
                -> Result<(),String> {
    let src = try!(convert_io_error(slurp(&path)));

    let instrs = try!(bfir::parse(&src));
    
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
    let initial_cells: Vec<u8> = state.cells.iter()
        .map(|x: &Wrapping<u8>| x.0).collect();

    let remaining_instrs = &instrs[state.instr_ptr..];
    let llvm_ir_raw = llvm::compile_to_ir(
        &path, &remaining_instrs.to_vec(), &initial_cells, state.cell_ptr as i32,
        &state.outputs);

    if dump_llvm {
        let llvm_ir = String::from_utf8_lossy(llvm_ir_raw.as_bytes());
        println!("{}", llvm_ir);
        return Ok(());
    }                        

    let bf_name = Path::new(path).file_name().unwrap();
    let obj_name = obj_file_name(bf_name.to_str().unwrap());

    let mut f = try!(convert_io_error(NamedTempFile::new()));

    let _ = f.write(llvm_ir_raw.as_bytes());

    // TODO: link as well.
    let llc_result = try!(
        convert_io_error(
            Command::new("llc")
                .arg("-O3").arg("-filetype=obj").arg(f.path())
                .arg("-o").arg(obj_name.to_owned())
                .output()));

    let llc_stderr = String::from_utf8_lossy(llc_result.stderr.as_slice());

    // TODO: it would be nice to have a function that wraps
    // Command::new so we don't have to inspect .status.success() and
    // wrap in Err as below.
    if llc_result.status.success() {
        // TODO: it would be cleaner to return this in Ok().
        println!("Wrote {}", obj_name);
        Ok(())
    } else {
        Err(llc_stderr.into_owned())
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

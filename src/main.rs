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
extern crate getopts;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::Read;
use std::num::Wrapping;
use std::path::Path;
use std::process::Command;
use getopts::{Options,Matches};
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
    println!("Usage: {} [options...] <BF source file> ", bin_name);
    println!("Examples:");
    println!("  {} foo.bf", bin_name);
    println!("  {} --dump-bf-ir foo.bf", bin_name);
    println!("  {} --dump-llvm foo.bf", bin_name);
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

fn compile_file(matches: &Matches) -> Result<(),String> {
    let ref path = matches.free[0];
    let src = try!(convert_io_error(slurp(path)));

    let mut instrs = try!(bfir::parse(&src));

    let opt_level = matches.opt_str("opt").unwrap_or(String::from("2"));
    if opt_level != "0" {
        instrs = optimize::optimize(instrs);
    }

    let state = if opt_level == "2" {
        execution::execute(&instrs, execution::MAX_STEPS)
    } else {
        execution::ExecutionState {
            instr_ptr: 0,
            cells: vec![Wrapping(0); bounds::highest_cell_index(&instrs) + 1],
            cell_ptr: 0,
            outputs: vec![]
        }
    };
    let initial_cells: Vec<u8> = state.cells.iter()
        .map(|x: &Wrapping<u8>| x.0).collect();

    let remaining_instrs = &instrs[state.instr_ptr..];

    if matches.opt_present("dump-bf-ir") {
        if remaining_instrs.is_empty() {
            println!("(optimized out)");
        }
        
        for instr in remaining_instrs {
            println!("{}", instr);
        }
        return Ok(());
    }

    let llvm_ir_raw = llvm::compile_to_ir(
        path, &remaining_instrs.to_vec(), &initial_cells, state.cell_ptr as i32,
        &state.outputs);

    if matches.opt_present("dump-llvm") {
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

    let mut opts = Options::new();
    
    opts.optflag("h", "help", "show usage");
    opts.optflag("", "dump-llvm", "print LLVM IR generated");
    opts.optflag("", "dump-bf-ir", "print BF IR generated");

    opts.optopt("O", "opt", "optimization level (0, 1 or 2)", "LEVEL");
    
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(_) => {
            print_usage(&args[0]);
            std::process::exit(1);
        }
    };

    if matches.free.len() != 1 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    match compile_file(&matches) {
        Ok(_) => {}
        Err(e) => {
            // TODO: this should go to stderr.
            println!("{}", e);
            std::process::exit(2);
        }
    }
}

#![warn(trivial_numeric_casts)]

// option_unwrap_used is specific to clippy. However, we don't want to
// add clippy to the build requirements, so we build without it and
// ignore any warnings about rustc not recognising clippy's lints.
#![allow(unknown_lints)]

// TODO: enable this warning and cleanup.
#![allow(option_unwrap_used)]

extern crate libc;
extern crate llvm_sys;
extern crate itertools;
extern crate quickcheck;
extern crate rand;
extern crate tempfile;
extern crate getopts;
extern crate ansi_term;

#[macro_use] extern crate matches;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::Read;
use std::num::Wrapping;
use std::path::Path;
use std::process::Command;
use getopts::{Options, Matches};
use tempfile::NamedTempFile;
use diagnostics::{Info,Level};

mod bfir;
mod llvm;
mod peephole;
mod bounds;
mod execution;
mod diagnostics;

#[cfg(test)]
mod peephole_tests;
#[cfg(test)]
mod llvm_tests;

/// Read the contents of the file at path, and return a string of its
/// contents. Return a diagnostic if we can't open or read the file.
fn slurp(path: &str) -> Result<String, Info> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(message) => {
            return Err(Info {
                level: Level::Error,
                filename: path.to_owned(),
                message: format!("{}", message)
            })
        }
    };

    let mut contents = String::new();

    match file.read_to_string(&mut contents) {
        Ok(_) => {
            Ok(contents)
        }
        Err(message) => {
            Err(Info {
                level: Level::Error,
                filename: path.to_owned(),
                message: format!("{}", message)
            })
        }
    }
}

/// Convert "foo.bf" to "foo".
#[allow(deprecated)] // .connect is in stable 1.2, but beta has deprecated it.
fn executable_name(bf_file_name: &str) -> String {
    let mut name_parts: Vec<_> = bf_file_name.split('.').collect();
    let parts_len = name_parts.len();
    if parts_len > 1 {
        name_parts.pop();
    }

    name_parts.connect(".")
}

fn print_usage(bin_name: &str, opts: Options) {
    let brief = format!("Usage: {} <BF source file> [options]", bin_name);
    print!("{}", opts.usage(&brief));
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

/// Execute the CLI command specified. Return Err if the command isn't
/// on $PATH, or if the command returned a non-zero exit code.
fn shell_command(command: &str, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new(command);
    for arg in args {
        c.arg(arg);
    }

    match c.output() {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                Ok((*stdout).to_owned())
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                Err((*stderr).to_owned())
            }
        }
        Err(_) => {
            Err(format!("Could not execute '{}'. Is it on $PATH?", command))
        }
    }
}

fn compile_file(matches: &Matches) -> Result<(), String> {
    let path = &matches.free[0];

    let src = match slurp(path) {
        Ok(src) => src,
        Err(info) => {
            return Err(format!("{}", info));
        }
    };

    let mut instrs = match bfir::parse(path, &src) {
        Ok(instrs) => {
            instrs
        }
        Err(info) => {
            return Err(format!("{}", info));
        }
    };
    
    let opt_level = matches.opt_str("opt").unwrap_or(String::from("2"));
    if opt_level != "0" {
        instrs = peephole::optimize(instrs);
    }

    if matches.opt_present("dump-ir") {
        for instr in instrs.iter() {
            println!("{}", instr);
        }
        return Ok(());
    }

    let state = if opt_level == "2" {
        execution::execute(&instrs, execution::MAX_STEPS)
    } else {
        execution::ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); bounds::highest_cell_index(&instrs) + 1],
            cell_ptr: 0,
            outputs: vec![],
        }
    };

    let llvm_ir_raw = llvm::compile_to_ir(path, &instrs, &state);

    if matches.opt_present("dump-llvm") {
        let llvm_ir = String::from_utf8_lossy(llvm_ir_raw.as_bytes());
        println!("{}", llvm_ir);
        return Ok(());
    }

    // Write the LLVM IR to a temporary file.
    let mut llvm_ir_file = try!(convert_io_error(NamedTempFile::new()));
    let _ = llvm_ir_file.write(llvm_ir_raw.as_bytes());

    // Compile the LLVM IR to a temporary object file.
    let object_file = try!(convert_io_error(NamedTempFile::new()));

    let llvm_opt_arg = format!("-O{}",
                               matches.opt_str("llvm-opt").unwrap_or(String::from("3")));

    let llc_args = [&llvm_opt_arg[..],
                    "-filetype=obj",
                    llvm_ir_file.path().to_str().expect("path not valid utf-8"),
                    "-o",
                    object_file.path().to_str().expect("path not valid utf-8")];
    try!(shell_command("llc", &llc_args[..]));

    // TODO: do path munging in executable_name().
    let bf_name = Path::new(path).file_name().unwrap();
    let output_name = executable_name(bf_name.to_str().unwrap());

    // Link the object file.
    let clang_args = [&llvm_opt_arg[..],
                      object_file.path().to_str().expect("path not valid utf-8"),
                      "-o",
                      &output_name[..]];
    try!(shell_command("clang", &clang_args[..]));

    // Strip the executable.
    let strip_args = ["-s", &output_name[..]];
    try!(shell_command("strip", &strip_args[..]));

    Ok(())
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    let args: Vec<_> = env::args().collect();

    let mut opts = Options::new();

    opts.optflag("h", "help", "show usage");
    opts.optflag("", "dump-llvm", "print LLVM IR generated");
    opts.optflag("", "dump-ir", "print BF IR generated");

    opts.optopt("O", "opt", "optimization level (0 to 2)", "LEVEL");
    opts.optopt("",
                "llvm-opt",
                "LLVM optimization level (0 to 3)",
                "LEVEL");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => {
            m
        }
        Err(_) => {
            print_usage(&args[0], opts);
            std::process::exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(&args[0], opts);
        return;
    }

    if matches.free.len() != 1 {
        print_usage(&args[0], opts);
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

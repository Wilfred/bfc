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

#[macro_use]
extern crate matches;

use std::env;
use std::fs::File;
use std::io::prelude::Read;
use std::num::Wrapping;
use std::path::Path;
use std::process::Command;
use getopts::{Options, Matches};
use tempfile::NamedTempFile;
use diagnostics::{Info, Level};

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
                message: format!("{}", message),
                position: None,
                source: None,
            })
        }
    };

    let mut contents = String::new();

    match file.read_to_string(&mut contents) {
        Ok(_) => Ok(contents),
        Err(message) => {
            Err(Info {
                level: Level::Error,
                filename: path.to_owned(),
                message: format!("{}", message),
                position: None,
                source: None,
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
        Ok(value) => Ok(value),
        Err(e) => Err(format!("{}", e)),
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
        Err(_) => Err(format!("Could not execute '{}'. Is it on $PATH?", command)),
    }
}

// TODO: return a Vec<Info> that may contain warnings or errors,
// instead of printing in lots of different place shere.
fn compile_file(matches: &Matches) -> Result<(), String> {
    let path = &matches.free[0];

    let src = match slurp(path) {
        Ok(src) => src,
        Err(info) => {
            return Err(format!("{}", info));
        }
    };

    let mut instrs = match bfir::parse(&src) {
        Ok(instrs) => instrs,
        Err(parse_error) => {
            let info = Info {
                level: Level::Error,
                filename: path.to_owned(),
                message: parse_error.message,
                position: Some(parse_error.position),
                source: Some(src),
            };
            return Err(format!("{}", info));
        }
    };

    let opt_level = matches.opt_str("opt").unwrap_or(String::from("2"));
    if opt_level != "0" {
        let (opt_instrs, warnings) = peephole::optimize(instrs);
        instrs = opt_instrs;

        for warning in warnings {
            let info = Info {
                level: Level::Warning,
                filename: path.to_owned(),
                message: warning.message,
                position: warning.position,
                source: Some(src.clone()),
            };
            println!("{}", info);
        }
    }

    if matches.opt_present("dump-ir") {
        for instr in &instrs {
            println!("{}", instr);
        }
        return Ok(());
    }

    let (state, warning) = if opt_level == "2" {
        execution::execute(&instrs, execution::MAX_STEPS)
    } else {
        (execution::ExecutionState {
            start_instr: Some(&instrs[0]),
            cells: vec![Wrapping(0); bounds::highest_cell_index(&instrs) + 1],
            cell_ptr: 0,
            outputs: vec![],
        },
         None)
    };

    if let Some(warning) = warning {
        let info = Info {
            level: Level::Warning,
            filename: path.to_owned(),
            message: warning.message,
            position: warning.position,
            source: Some(src),
        };
        println!("{}", info);
    }

    let mut llvm_module = llvm::compile_to_module(path, &instrs, &state);

    if matches.opt_present("dump-llvm") {
        let llvm_ir_cstr = llvm_module.to_cstring();
        let llvm_ir = String::from_utf8_lossy(llvm_ir_cstr.as_bytes());
        println!("{}", llvm_ir);
        return Ok(());
    }

    let llvm_opt_raw = matches.opt_str("llvm-opt").unwrap_or("3".to_owned());
    let mut llvm_opt = llvm_opt_raw.parse::<i64>().unwrap_or(3);
    if llvm_opt < 0 || llvm_opt > 3 {
        // TODO: warn on unrecognised input.
        llvm_opt = 3;
    }

    llvm::optimise_ir(&mut llvm_module, llvm_opt);

    // Compile the LLVM IR to a temporary object file.
    let object_file = try!(convert_io_error(NamedTempFile::new()));
    let obj_file_path = object_file.path().to_str().expect("path not valid utf-8");
    llvm::write_object_file(&mut llvm_module, &obj_file_path);

    // TODO: do path munging in executable_name().
    let bf_name = Path::new(path).file_name().unwrap();
    let output_name = executable_name(bf_name.to_str().unwrap());

    // Link the object file.
    let clang_args = [obj_file_path, "-o", &output_name[..]];
    // TODO: use cc instead of clang here.
    // TODO: factor out linking, writing to smaller functions.
    try!(shell_command("clang", &clang_args[..]));

    // Strip the executable.
    let strip_args = ["-s", &output_name[..]];
    try!(shell_command("strip", &strip_args[..]));

    Ok(())
}

fn main() {
    let args: Vec<_> = env::args().collect();

    let mut opts = Options::new();

    opts.optflag("h", "help", "show usage");
    opts.optflag("", "dump-llvm", "print LLVM IR generated");
    opts.optflag("", "dump-ir", "print BF IR generated");

    opts.optopt("O", "opt", "optimization level (0 to 2)", "LEVEL");
    opts.optopt("", "llvm-opt", "LLVM optimization level (0 to 3)", "LEVEL");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
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

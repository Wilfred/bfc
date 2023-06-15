#![warn(trivial_numeric_casts)]

//! bfc is a highly optimising compiler for BF.

use ariadne::{Label, Report, ReportKind, Source};
use bfir::Position;
use clap::builder::ValueParser;
use clap::command;
use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use clap::ValueHint;
use std::env;
use std::fs::File;
use std::io::prelude::Read;
use std::path::Path;
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[cfg(test)]
use pretty_assertions::assert_eq;

mod bfir;
mod bounds;
mod diagnostics;
mod execution;
mod llvm;
mod peephole;
mod shell;

#[cfg(test)]
mod llvm_tests;
#[cfg(test)]
mod peephole_tests;
#[cfg(test)]
mod soundness_tests;

/// Read the contents of the file at path, and return a string of its
/// contents. Return a diagnostic if we can't open or read the file.
fn slurp(path: &Path) -> Result<String, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(message) => {
            return Err(format!("{}: {}", path.display(), message));
        }
    };

    let mut contents = String::new();

    match file.read_to_string(&mut contents) {
        Ok(_) => Ok(contents),
        Err(message) => Err(format!("{} {}", path.display(), message)),
    }
}

/// Convert "foo.bf" to "foo".
fn executable_name(bf_path: &Path) -> String {
    let bf_file_name = bf_path.file_name().unwrap().to_str().unwrap();

    let mut name_parts: Vec<_> = bf_file_name.split('.').collect();
    let parts_len = name_parts.len();
    if parts_len > 1 {
        name_parts.pop();
    }

    name_parts.join(".")
}

#[test]
fn executable_name_bf() {
    assert_eq!(executable_name(&PathBuf::from("foo.bf")), "foo");
}

#[test]
fn executable_name_b() {
    assert_eq!(executable_name(&PathBuf::from("foo_bar.b")), "foo_bar");
}

#[test]
fn executable_name_relative_path() {
    assert_eq!(executable_name(&PathBuf::from("bar/baz.bf")), "baz");
}

fn convert_io_error<T>(result: Result<T, std::io::Error>) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(e) => Err(format!("{}", e)),
    }
}

fn compile_file(matches: &ArgMatches) -> Result<(), ()> {
    let path = matches
        .get_one::<PathBuf>("path")
        .expect("Required argument");

    let src = slurp(path).map_err(|e| {
        eprintln!("{}", e);
    })?;

    let mut instrs = match bfir::parse(&src) {
        Ok(instrs) => instrs,
        Err(bfir::ParseError { message, position }) => {
            let path_str = path.display().to_string();
            Report::build(ReportKind::Error, &path_str, position.start)
                .with_message("Parse error")
                .with_label(
                    Label::new((&path_str, position.start..position.end + 1))
                        .with_message(message.clone()),
                )
                .finish()
                .eprint((&path_str, Source::from(src.clone())))
                .unwrap();

            return Err(());
        }
    };

    let opt_level = matches.get_one::<String>("opt").expect("Required argument");
    if opt_level != "0" {
        let pass_specification = matches.get_one::<String>("passes");
        let (opt_instrs, warnings) = peephole::optimize(instrs, &pass_specification.cloned());
        instrs = opt_instrs;

        for diagnostics::Warning { message, position } in warnings {
            let path_str = path.display().to_string();
            let position = position.unwrap_or(Position { start: 0, end: 0 });
            Report::build(ReportKind::Warning, &path_str, position.start)
                .with_message("Suspicious code found during optimization")
                .with_label(
                    Label::new((&path_str, position.start..position.end + 1))
                        .with_message(message.clone()),
                )
                .finish()
                .eprint((&path_str, Source::from(src.clone())))
                .unwrap();
        }
    }

    if matches.get_flag("dump-ir") {
        for instr in &instrs {
            println!("{}", instr);
        }
        return Ok(());
    }

    let (state, execution_warning) = if opt_level == "2" {
        execution::execute(&instrs, execution::max_steps())
    } else {
        let mut init_state = execution::ExecutionState::initial(&instrs[..]);
        // TODO: this will crash on the empty program.
        init_state.start_instr = Some(&instrs[0]);
        (init_state, None)
    };

    if let Some(diagnostics::Warning { message, position }) = execution_warning {
        let path_str = path.display().to_string();
        let position = position.unwrap_or(Position { start: 0, end: 0 });

        Report::build(ReportKind::Warning, &path_str, position.start)
            .with_message("Invalid result during compiletime execution")
            .with_label(
                Label::new((&path_str, position.start..position.end + 1))
                    .with_message(message.clone()),
            )
            .finish()
            .eprint((&path_str, Source::from(src.clone())))
            .unwrap();
    }

    llvm::init_llvm();
    let target_triple = matches.get_one::<String>("target");
    let mut llvm_module = llvm::compile_to_module(
        &path.display().to_string(),
        target_triple.cloned(),
        &instrs,
        &state,
    );

    if matches.get_flag("dump-llvm") {
        let llvm_ir_cstr = llvm_module.to_cstring();
        let llvm_ir = String::from_utf8_lossy(llvm_ir_cstr.as_bytes());
        println!("{}", llvm_ir);
        return Ok(());
    }

    let llvm_opt_raw = matches
        .get_one::<String>("llvm-opt")
        .expect("Required argument");
    let llvm_opt = llvm_opt_raw.parse::<i64>().expect("Validated by clap");
    llvm::optimise_ir(&mut llvm_module, llvm_opt);

    // Compile the LLVM IR to a temporary object file.
    let object_file = convert_io_error(NamedTempFile::new()).map_err(|e| {
        eprintln!("{}", e);
    })?;

    let obj_file_path = object_file.path().to_str().expect("path not valid utf-8");
    llvm::write_object_file(&mut llvm_module, obj_file_path).map_err(|e| {
        eprintln!("{}", e);
    })?;

    let output_name = executable_name(path);
    link_object_file(obj_file_path, &output_name, target_triple.cloned()).map_err(|e| {
        eprintln!("{}", e);
    })?;

    let strip_opt = matches.get_one::<String>("strip").expect("Has default");
    if strip_opt == "yes" {
        strip_executable(&output_name).map_err(|e| {
            eprintln!("{}", e);
        })?;
    }

    Ok(())
}

fn link_object_file(
    object_file_path: &str,
    executable_path: &str,
    target_triple: Option<String>,
) -> Result<(), String> {
    // Link the object file.
    let clang_args = if let Some(ref target_triple) = target_triple {
        vec![
            object_file_path,
            "-target",
            target_triple,
            "-o",
            executable_path,
        ]
    } else {
        vec![object_file_path, "-o", executable_path]
    };

    shell::run_shell_command("clang", &clang_args[..])
}

fn strip_executable(executable_path: &str) -> Result<(), String> {
    let strip_args = match std::env::consts::OS {
        "macos" => vec![executable_path],
        _ => vec!["-s", executable_path],
    };
    shell::run_shell_command("strip", &strip_args[..])
}

fn main() {
    let default_triple_cstring = llvm::get_default_target_triple();
    let default_triple = default_triple_cstring.to_str().unwrap();

    let matches = command!()
        .arg(
            Arg::new("path")
                .value_name("SOURCE_FILE")
                .value_hint(ValueHint::FilePath)
                .help("The path to the brainfuck program to compile")
                .value_parser(ValueParser::path_buf())
                .required(true),
        )
        .arg(
            Arg::new("opt")
                .short('O')
                .long("opt")
                .value_name("LEVEL")
                .help("Optimization level")
                .value_parser(["0", "1", "2"])
                .default_value("2"),
        )
        .arg(
            Arg::new("llvm-opt")
                .long("llvm-opt")
                .value_name("LEVEL")
                .help("LLVM optimization level")
                .value_parser(["0", "1", "2", "3"])
                .default_value("3"),
        )
        .arg(
            Arg::new("passes")
                .long("passes")
                .value_name("PASS-SPECIFICATION")
                .help("Limit bfc optimizations to those specified"),
        )
        .arg(
            Arg::new("strip")
                .long("strip")
                .value_name("yes|no")
                .help("Strip symbols from the binary")
                .value_parser(["yes", "no"])
                .default_value("yes"),
        )
        .arg(
            Arg::new("target")
                .long("target")
                .value_name("TARGET")
                .help("LLVM target triple")
                .default_value(default_triple.to_string()),
        )
        .arg(
            Arg::new("dump-llvm")
                .long("dump-llvm")
                .action(ArgAction::SetTrue)
                .action(ArgAction::SetTrue)
                .help("Print the LLVM IR generated"),
        )
        .arg(
            Arg::new("dump-ir")
                .long("dump-ir")
                .action(ArgAction::SetTrue)
                .help("Print the BF IR generated"),
        )
        .get_matches();

    match compile_file(&matches) {
        Ok(_) => {}
        Err(()) => {
            std::process::exit(2);
        }
    }
}

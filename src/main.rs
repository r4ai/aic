mod ast;
mod codegen;
mod parser;

use anyhow::Result;
use clap::Parser;
use inkwell::context::Context;
use std::{fs, path::PathBuf};

/// A simple integer-only compiler
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file to compile
    #[arg(short, long)]
    input: PathBuf,

    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Emit LLVM IR instead of an object file
    #[arg(long)]
    emit_llvm: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read the input file
    let input = fs::read_to_string(&args.input)?;

    // Parse the input
    let program = parser::parse(&input)?;
    println!("Parsed AST: {:#?}", program);

    // Generate code
    let context = Context::create();
    let module_name = args
        .input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("module");

    let codegen = codegen::CodeGen::new(&context, module_name);
    codegen.compile(&program)?;

    // Output
    if args.emit_llvm {
        // Print LLVM IR
        println!("Generated LLVM IR:");
        println!("{}", codegen.print_ir());
    } else {
        // Compile to an object file
        let output = args
            .output
            .unwrap_or_else(|| PathBuf::from(format!("{}.o", module_name)));

        codegen.compile_to_file(output.to_str().unwrap())?;
        println!("Compiled to {}", output.display());
    }

    Ok(())
}

mod ast;
mod codegen;
mod parser;
mod token;

use anyhow::Result;
use ariadne::{Report, ReportKind};
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
    let program = match parser::parse(&input).into_result() {
        Ok(program) => program,
        Err(errors) => {
            for err in errors {
                Report::build(ReportKind::Error, ((), err.span().into_range()))
                    .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                    .with_code(3)
                    .with_message(err.to_string())
                    .with_label(
                        ariadne::Label::new(((), err.span().into_range()))
                            .with_message(err.reason().to_string())
                            .with_color(ariadne::Color::Red),
                    )
                    .finish()
                    .eprint(ariadne::Source::from(&input))
                    .unwrap();
            }
            return Err(anyhow::anyhow!("Failed to parse input"));
        }
    };
    println!("Parsed AST:\n {:#?}", program);

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

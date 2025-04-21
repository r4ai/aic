use anyhow::Result;
use inkwell::{
    OptimizationLevel,
    context::Context,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
};

use crate::ast::{BinOp, Expr, Program, UnaryOp};

/// Code generator for compiling AST to LLVM IR
pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: inkwell::builder::Builder<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
    /// Create a new code generator
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
        }
    }

    /// Compile the program and return the resulting module
    pub fn compile(&self, program: &Program) -> Result<()> {
        // Create a main function
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        // Generate code for the expression
        let result = self.gen_expr(&program.expr)?;

        // Return the result of the expression
        self.builder
            .build_return(Some(&result))
            .map_err(|e| anyhow::anyhow!("Failed to build return: {}", e))?;

        // Verify the module
        if self.module.verify().is_err() {
            return Err(anyhow::anyhow!("Module verification failed"));
        }

        Ok(())
    }

    /// Generate LLVM IR for an expression
    fn gen_expr(&self, expr: &Expr) -> Result<inkwell::values::IntValue<'ctx>> {
        match expr {
            Expr::IntLit(value) => {
                let i32_type = self.context.i32_type();
                Ok(i32_type.const_int(*value as u64, false))
            }
            Expr::BinOp { lhs, op, rhs } => {
                let lhs = self.gen_expr(lhs)?;
                let rhs = self.gen_expr(rhs)?;

                match op {
                    BinOp::Add => self
                        .builder
                        .build_int_add(lhs, rhs, "addtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build add: {}", e)),
                    BinOp::Sub => self
                        .builder
                        .build_int_sub(lhs, rhs, "subtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build sub: {}", e)),
                    BinOp::Mul => self
                        .builder
                        .build_int_mul(lhs, rhs, "multmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build mul: {}", e)),
                    BinOp::Div => self
                        .builder
                        .build_int_signed_div(lhs, rhs, "divtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build div: {}", e)),
                }
            }
            Expr::UnaryOp {
                op: UnaryOp::Neg,
                expr,
            } => {
                let value = self.gen_expr(expr)?;
                let zero = self.context.i32_type().const_int(0, false);
                self.builder
                    .build_int_sub(zero, value, "negtmp")
                    .map_err(|e| anyhow::anyhow!("Failed to build negation: {}", e))
            }
        }
    }

    /// Output the LLVM IR as a string
    pub fn print_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Compile to a native executable file
    pub fn compile_to_file(&self, filename: &str) -> Result<()> {
        // Initialize the target
        Target::initialize_all(&InitializationConfig::default());

        // Get the host target triple
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| anyhow::anyhow!("Failed to get target from triple: {}", e))?;

        // Create a target machine
        let target_machine = target
            .create_target_machine(
                &triple,
                &TargetMachine::get_host_cpu_name().to_string(),
                &TargetMachine::get_host_cpu_features().to_string(),
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| anyhow::anyhow!("Failed to create target machine"))?;

        // Emit object file
        target_machine
            .write_to_file(
                &self.module,
                inkwell::targets::FileType::Object,
                filename.as_ref(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to write object file: {}", e))?;

        Ok(())
    }
}

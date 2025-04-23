use anyhow::{Result, bail};
use inkwell::{
    OptimizationLevel,
    context::Context,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    types::BasicMetadataTypeEnum,
};

use crate::ast;

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
    pub fn compile(&self, program: &ast::Program) -> Result<()> {
        // Create a main function
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        // Generate code for the program
        self.gen_program(program)?;

        // Verify the module
        if self.module.verify().is_err() {
            return Err(anyhow::anyhow!("Module verification failed"));
        }

        Ok(())
    }

    /// Generate LLVM IR for a program
    pub fn gen_program(&self, program: &ast::Program) -> Result<()> {
        for stmt in &program.statements {
            self.gen_stmt(stmt)?;
        }
        Ok(())
    }

    /// Generate LLVM IR for a statement
    fn gen_stmt(&self, stmt: &ast::Stmt) -> Result<()> {
        match stmt {
            ast::Stmt::FnDecl {
                name,
                params,
                r#type,
                body,
            } => {
                let initial_pos = self.builder.get_insert_block().unwrap();

                // Create function type
                let param_types: Vec<BasicMetadataTypeEnum> = params
                    .iter()
                    .map(|param| match param.r#type {
                        ast::Type::Void => bail!("Void type not allowed in function parameters"),
                        ast::Type::String => todo!("String type not implemented"),
                        ast::Type::I32 => Ok(self.context.i32_type().into()),
                        ast::Type::I64 => Ok(self.context.i64_type().into()),
                        ast::Type::F32 => Ok(self.context.f32_type().into()),
                        ast::Type::F64 => Ok(self.context.f64_type().into()),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let fn_type = match r#type {
                    ast::Type::Void => self.context.void_type().fn_type(&param_types, false),
                    ast::Type::String => todo!("String type not implemented"),
                    ast::Type::I32 => self.context.i32_type().fn_type(&param_types, false),
                    ast::Type::I64 => self.context.i64_type().fn_type(&param_types, false),
                    ast::Type::F32 => self.context.f32_type().fn_type(&param_types, false),
                    ast::Type::F64 => self.context.f64_type().fn_type(&param_types, false),
                };
                let function = self.module.add_function(name, fn_type, None);

                // Create basic block for the function
                let basic_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(basic_block);

                // Generate code for the function body
                for stmt in body.iter() {
                    self.gen_stmt(stmt)?;
                }

                // Generate implicit return
                let has_return = body.iter().any(|s| matches!(&s, ast::Stmt::Expr { .. }));
                if !has_return {
                    // If the function has no return statement, return void
                    self.builder
                        .build_return(None)
                        .map_err(|e| anyhow::anyhow!("Failed to build return: {}", e))?;
                }

                // Change the position of the builder back to the initial position
                self.builder.position_at_end(initial_pos);
            }
            ast::Stmt::ExprStmt { expr } => {
                self.gen_expr(expr)?;
            }
            ast::Stmt::Expr { expr } => {
                let value = self.gen_expr(expr)?;

                // Stmt::Expr can only exist at the end of a block, so it's safe to return this value
                // The fact that it only exists at the end is defined in the parser's grammar, so we don't need to check it again here
                self.builder
                    .build_return(Some(&value))
                    .map_err(|e| anyhow::anyhow!("Failed to build return: {}", e))?;
            }
        }
        Ok(())
    }

    /// Generate LLVM IR for an expression
    fn gen_expr(&self, expr: &ast::Expr) -> Result<inkwell::values::IntValue<'ctx>> {
        match expr {
            ast::Expr::IntLit(value) => {
                let i32_type = self.context.i32_type();
                Ok(i32_type.const_int(*value as u64, false))
            }
            ast::Expr::BinOp { lhs, op, rhs } => {
                let lhs = self.gen_expr(lhs)?;
                let rhs = self.gen_expr(rhs)?;

                match op {
                    ast::BinOp::Add => self
                        .builder
                        .build_int_add(lhs, rhs, "addtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build add: {}", e)),
                    ast::BinOp::Sub => self
                        .builder
                        .build_int_sub(lhs, rhs, "subtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build sub: {}", e)),
                    ast::BinOp::Mul => self
                        .builder
                        .build_int_mul(lhs, rhs, "multmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build mul: {}", e)),
                    ast::BinOp::Div => self
                        .builder
                        .build_int_signed_div(lhs, rhs, "divtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build div: {}", e)),
                }
            }
            ast::Expr::UnaryOp {
                op: ast::UnaryOp::Neg,
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

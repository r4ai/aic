use std::collections::HashMap;

use anyhow::{Result, bail};
use inkwell::{
    OptimizationLevel,
    context::Context,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    types::BasicMetadataTypeEnum,
    values::BasicValueEnum,
};

use crate::ast;

pub struct Env<'ctx> {
    scopes: Vec<HashMap<&'ctx str, Option<inkwell::values::BasicValueEnum<'ctx>>>>,
}

impl<'ctx> Env<'ctx> {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(
        &mut self,
        name: &'ctx str,
        value: inkwell::values::BasicValueEnum<'ctx>,
    ) -> Result<()> {
        if self
            .scopes
            .last_mut()
            .unwrap()
            .insert(name, Some(value))
            .is_some()
        {
            bail!("Variable '{}' already declared", name);
        }
        Ok(())
    }

    fn resolve_var(&self, name: &'ctx str) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                if let Some(value) = value {
                    return Ok(*value);
                } else {
                    bail!("Variable '{}' is not initialized", name);
                }
            }
        }
        bail!("Variable '{}' not found", name);
    }
}

/// Code generator for compiling AST to LLVM IR
pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: inkwell::builder::Builder<'ctx>,
    env: Env<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
    /// Create a new code generator
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let env = Env::new();
        Self {
            context,
            module,
            builder,
            env,
        }
    }

    /// Compile the program and return the resulting module
    pub fn compile(&mut self, program: &'ctx ast::Program) -> Result<()> {
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
            eprintln!("LLVM IR:\n {}\n", self.module.print_to_string().to_string());
            eprintln!("Error message:\n {:?}\n", self.module.verify().unwrap_err());
            return Err(anyhow::anyhow!("Module verification failed"));
        }

        Ok(())
    }

    /// Generate LLVM IR for a program
    pub fn gen_program(&mut self, program: &'ctx ast::Program) -> Result<()> {
        self.gen_block(&program.statements)
    }

    /// Generate LLVM IR for a block
    pub fn gen_block(&mut self, stmts: &'ctx Vec<ast::Stmt>) -> Result<()> {
        self.env.push_scope();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last_stmt = i == stmts.len() - 1;
            self.gen_stmt(stmt, is_last_stmt)?;
        }
        self.env.pop_scope();
        Ok(())
    }

    /// Generate LLVM IR for a statement
    fn gen_stmt(&mut self, stmt: &'ctx ast::Stmt, is_last_stmt: bool) -> Result<()> {
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

                // Set the function parameters
                for (i, param) in function.get_param_iter().enumerate() {
                    let name = params[i].name;
                    self.env.declare_var(name, param).map_err(|e| {
                        anyhow::anyhow!("Failed to declare parameter '{}': {}", name, e)
                    })?;
                }

                // Create basic block for the function
                let basic_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(basic_block);

                // Generate code for the function body
                self.gen_block(body)?;

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
            ast::Stmt::VarDecl {
                name,
                r#type,
                value,
            } => {
                // Generate code for the variable declaration
                let value: BasicValueEnum = if let Some(value) = value {
                    self.gen_expr(value)?
                } else {
                    // If no value is provided, use a default value
                    match r#type {
                        Some(ast::Type::I32) => self.context.i32_type().const_zero().into(),
                        Some(ast::Type::I64) => self.context.i64_type().const_zero().into(),
                        Some(ast::Type::F32) => self.context.f32_type().const_zero().into(),
                        Some(ast::Type::F64) => self.context.f64_type().const_zero().into(),
                        _ => bail!("Unsupported type for variable declaration"),
                    }
                };

                // Declare the variable in the current scope
                self.env
                    .declare_var(name, value)
                    .map_err(|e| anyhow::anyhow!("Failed to declare variable '{}': {}", name, e))?;
            }
            ast::Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Get the current function
                let function = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                // Create basic blocks for the if branches
                let then_block = self.context.append_basic_block(function, "then");
                let else_block = if else_branch.is_some() {
                    Some(self.context.append_basic_block(function, "else"))
                } else {
                    None
                };
                let merge_block = self.context.append_basic_block(function, "ifcont");

                // Generate condition code
                let condition_value = self.gen_expr(condition)?;

                // Convert the condition to i1 (boolean) type
                let condition_value = if condition_value.is_int_value() {
                    self.builder.build_int_compare(
                        inkwell::IntPredicate::NE,
                        condition_value.into_int_value(),
                        self.context.i32_type().const_zero(),
                        "ifcond",
                    )?
                } else {
                    // Todo support other types
                    bail!("Condition must be an i1 (boolean) value");
                };

                // Build the conditional branch
                self.builder
                    .build_conditional_branch(
                        condition_value,
                        then_block,
                        if let Some(else_block) = else_block {
                            else_block
                        } else {
                            merge_block
                        },
                    )
                    .map_err(|e| anyhow::anyhow!("Failed to build conditional branch: {}", e))?;

                // Generate 'then' branch code
                self.builder.position_at_end(then_block);
                self.gen_block(then_branch)?;

                // Jump to the merge block if there's no terminator (like a return)
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to build unconditional branch: {}", e)
                        })?;
                }

                // Generate 'else' branch code if it exists
                if let Some(else_branch) = else_branch {
                    self.builder.position_at_end(else_block.unwrap());
                    self.gen_block(else_branch)?;

                    // Jump to the merge block if there's no terminator
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        self.builder
                            .build_unconditional_branch(merge_block)
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to build unconditional branch: {}", e)
                            })?;
                    }
                }

                if is_last_stmt {
                    merge_block.remove_from_function().map_err(|_| {
                        anyhow::anyhow!("Failed to remove merge block from function")
                    })?;
                }

                // Position the builder at the merge block
                self.builder.position_at_end(merge_block);
            }
        }
        Ok(())
    }

    /// Generate LLVM IR for an expression
    fn gen_expr(&self, expr: &'ctx ast::Expr) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        match expr {
            ast::Expr::IntLit(value) => {
                let i32_type = self.context.i32_type();
                Ok(i32_type.const_int(*value as u64, false).into())
            }
            ast::Expr::BinOp { lhs, op, rhs } => {
                let lhs = self.gen_expr(lhs)?;
                let rhs = self.gen_expr(rhs)?;

                if lhs.get_type() != rhs.get_type() {
                    bail!("Type mismatch in binary operation");
                }

                if !lhs.is_int_value() || !rhs.is_int_value() {
                    bail!("Binary operation only supports integer values");
                }

                let lhs = lhs.into_int_value();
                let rhs = rhs.into_int_value();

                match op {
                    ast::BinOp::Add => self
                        .builder
                        .build_int_add(lhs, rhs, "addtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build add: {}", e))
                        .map(|v| v.into()),
                    ast::BinOp::Sub => self
                        .builder
                        .build_int_sub(lhs, rhs, "subtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build sub: {}", e))
                        .map(|v| v.into()),
                    ast::BinOp::Mul => self
                        .builder
                        .build_int_mul(lhs, rhs, "multmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build mul: {}", e))
                        .map(|v| v.into()),
                    ast::BinOp::Div => self
                        .builder
                        .build_int_signed_div(lhs, rhs, "divtmp")
                        .map_err(|e| anyhow::anyhow!("Failed to build div: {}", e))
                        .map(|v| v.into()),
                }
            }
            ast::Expr::UnaryOp {
                op: ast::UnaryOp::Neg,
                expr,
            } => {
                let value = self.gen_expr(expr)?;
                if !value.is_int_value() {
                    bail!("Unary negation only supports integer values");
                }
                let value = value.into_int_value();

                let zero = self.context.i32_type().const_int(0, false);
                self.builder
                    .build_int_sub(zero, value, "negtmp")
                    .map_err(|e| anyhow::anyhow!("Failed to build negation: {}", e))
                    .map(|v| v.into())
            }
            ast::Expr::FnCall { name, args } => {
                // Look up the function by name
                let function = self
                    .module
                    .get_function(name)
                    .ok_or_else(|| anyhow::anyhow!("Function '{}' not found", name))?;
                // Generate code for each argument
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.gen_expr(arg)?);
                }
                // Build the call
                let call_site = self.builder.build_call(
                    function,
                    &arg_values.iter().map(|v| (*v).into()).collect::<Vec<_>>(),
                    "calltmp",
                )?;
                // Assume all functions return i32 for now
                let ret_val = call_site.try_as_basic_value().left().unwrap();
                Ok(ret_val)
            }
            ast::Expr::VarRef { name } => {
                // Look up the variable by name
                let value = self
                    .env
                    .resolve_var(name)
                    .map_err(|e| anyhow::anyhow!("Variable '{}' not found: {}", name, e))?;
                Ok(value)
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

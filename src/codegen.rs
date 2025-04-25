use std::collections::HashMap;

use anyhow::{Result, bail};
use inkwell::{
    OptimizationLevel,
    context::Context,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum}, // Import BasicType trait
    values::{BasicValueEnum, PointerValue},
};

use crate::ast;

struct VariableInfo<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: BasicTypeEnum<'ctx>, // Store the type of the variable
    is_mutable: bool,
}

pub struct Env<'ctx> {
    scopes: Vec<HashMap<&'ctx str, VariableInfo<'ctx>>>,
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
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>, // Add type parameter
        is_mutable: bool,
    ) -> Result<()> {
        if self
            .scopes
            .last_mut()
            .unwrap()
            .insert(
                name,
                VariableInfo {
                    ptr,
                    ty,
                    is_mutable,
                },
            ) // Store the type
            .is_some()
        {
            bail!("Variable '{}' already declared in this scope", name);
        }
        Ok(())
    }

    fn resolve_var(&self, name: &'ctx str) -> Result<&VariableInfo<'ctx>> {
        for scope in self.scopes.iter().rev() {
            if let Some(var_info) = scope.get(name) {
                return Ok(var_info);
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
            eprintln!("LLVM IR:\n{}\n", self.module.print_to_string().to_string());
            eprintln!(
                "Error message:\n{}\n",
                self.module.verify().unwrap_err().to_string()
            );
            return Err(anyhow::anyhow!("Module verification failed"));
        }

        Ok(())
    }

    /// Generate LLVM IR for a program
    pub fn gen_program(&mut self, program: &'ctx ast::Program) -> Result<()> {
        self.gen_block(&program.statements, true)
    }

    /// Generate LLVM IR for a block
    pub fn gen_block(&mut self, stmts: &'ctx Vec<ast::Stmt>, is_last_block: bool) -> Result<()> {
        self.env.push_scope();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last_stmt = is_last_block && (i == stmts.len() - 1);
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
                    .map(|param| self.map_ast_type_to_llvm(param.r#type).map(|t| t.into()))
                    .collect::<Result<Vec<_>, _>>()?;

                let fn_type = match self.map_ast_type_to_llvm(*r#type) {
                    Ok(ty) => ty.fn_type(&param_types, false),
                    Err(_) if *r#type == ast::Type::Void => {
                        self.context.void_type().fn_type(&param_types, false)
                    }
                    Err(e) => return Err(e),
                };

                let function = self.module.add_function(name, fn_type, None);

                // Create basic block for the function
                let basic_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(basic_block);

                // Allocate space for parameters and store initial values
                self.env.push_scope(); // Push scope for function parameters
                for (i, param) in function.get_param_iter().enumerate() {
                    let ast_param = &params[i];
                    let param_type = self.map_ast_type_to_llvm(ast_param.r#type)?;
                    let alloca = self.builder.build_alloca(param_type, ast_param.name)?;
                    self.builder.build_store(alloca, param)?;
                    self.env
                        .declare_var(ast_param.name, alloca, param_type, false) // Pass param_type
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to declare parameter '{}': {}",
                                ast_param.name,
                                e
                            )
                        })?;
                }

                // Generate code for the function body
                self.gen_block(body, true)?;

                self.env.pop_scope(); // Pop scope for function parameters

                // Change the position of the builder back to the initial position
                self.builder.position_at_end(initial_pos);
            }
            ast::Stmt::Return { expr } => match expr {
                Some(expr) => {
                    let value = self.gen_expr(expr)?;
                    self.builder
                        .build_return(Some(&value))
                        .map_err(|e| anyhow::anyhow!("Failed to build return: {}", e))?;
                }
                None => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| anyhow::anyhow!("Failed to build return: {}", e))?;
                }
            },
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
            ast::Stmt::LetDecl {
                name,
                r#type,
                value,
            } => {
                let initial_value = if let Some(val_expr) = value {
                    self.gen_expr(val_expr)?
                } else {
                    bail!("Initial value required for let declaration");
                };

                let var_type = initial_value.get_type();
                if let Some(ty) = r#type {
                    let llvm_type = self.map_ast_type_to_llvm(*ty)?;
                    if var_type != llvm_type {
                        bail!(
                            "Type mismatch in let declaration: expected {:?}, found {:?}",
                            llvm_type,
                            var_type
                        );
                    }
                }

                let ptr = self.builder.build_alloca(var_type, name)?;
                self.builder.build_store(ptr, initial_value)?;

                // Declare the immutable variable in the current scope
                self.env
                    .declare_var(name, ptr, var_type, false) // Pass var_type
                    .map_err(|e| anyhow::anyhow!("Failed to declare variable '{}': {}", name, e))?;
            }
            ast::Stmt::VarDecl {
                name,
                r#type,
                value,
            } => {
                let initial_value = if let Some(val_expr) = value {
                    self.gen_expr(val_expr)?
                } else {
                    // Determine type and get default value if no initial value provided
                    let ty = r#type.ok_or_else(|| {
                        anyhow::anyhow!(
                            "Type annotation required for var declaration without initializer"
                        )
                    })?;
                    self.get_default_value(ty)?
                };

                let var_type = initial_value.get_type();
                let ptr = self.builder.build_alloca(var_type, name)?;
                self.builder.build_store(ptr, initial_value)?;

                // Declare the mutable variable in the current scope
                self.env
                    .declare_var(name, ptr, var_type, true) // Pass var_type
                    .map_err(|e| anyhow::anyhow!("Failed to declare variable '{}': {}", name, e))?;
            }
            ast::Stmt::Assign { name, value } => {
                let new_value = self.gen_expr(value)?;
                let var_info = self.env.resolve_var(name)?;

                if !var_info.is_mutable {
                    bail!("Cannot assign to immutable variable '{}'", name);
                }

                // Load the existing value's type to ensure type match
                let current_value =
                    self.builder
                        .build_load(var_info.ty, var_info.ptr, "loadtmp")?; // Use stored type
                if new_value.get_type() != current_value.get_type() {
                    bail!("Type mismatch in assignment to variable '{}'", name);
                }

                self.builder.build_store(var_info.ptr, new_value)?;
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
                    condition_value.into_int_value()
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
                self.gen_block(then_branch, is_last_stmt)?;

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
                    self.gen_block(else_branch, is_last_stmt)?;

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
            ast::Expr::BoolLit(value) => {
                // Boolean literals (true/false) are represented as i1 (1-bit integer) in LLVM
                let bool_type = self.context.bool_type();
                let bool_value = if *value {
                    bool_type.const_int(1, false)
                } else {
                    bool_type.const_zero()
                };
                Ok(bool_value.into())
            }
            ast::Expr::BinOp { lhs, op, rhs } => {
                let lhs = self.gen_expr(lhs)?;
                let rhs = self.gen_expr(rhs)?;

                // Handle comparison and logical operators
                match op {
                    // Equality operators
                    ast::BinOp::Equal | ast::BinOp::NotEqual => {
                        if lhs.get_type() != rhs.get_type() {
                            bail!("Type mismatch in equality operation");
                        }

                        if lhs.is_int_value() && rhs.is_int_value() {
                            let lhs_int = lhs.into_int_value();
                            let rhs_int = rhs.into_int_value();
                            let predicate = match op {
                                ast::BinOp::Equal => inkwell::IntPredicate::EQ,
                                ast::BinOp::NotEqual => inkwell::IntPredicate::NE,
                                _ => unreachable!(),
                            };
                            self.builder
                                .build_int_compare(predicate, lhs_int, rhs_int, "cmptmp")
                                .map_err(|e| anyhow::anyhow!("Failed to build comparison: {}", e))
                                .map(|v| v.into())
                        } else {
                            bail!("Equality operation only supports integer values for now");
                        }
                    }
                    // Comparison operators
                    ast::BinOp::LessThan
                    | ast::BinOp::LessThanOrEqual
                    | ast::BinOp::GreaterThan
                    | ast::BinOp::GreaterThanOrEqual => {
                        if lhs.get_type() != rhs.get_type() {
                            bail!("Type mismatch in comparison operation");
                        }

                        if lhs.is_int_value() && rhs.is_int_value() {
                            let lhs_int = lhs.into_int_value();
                            let rhs_int = rhs.into_int_value();
                            let predicate = match op {
                                ast::BinOp::LessThan => inkwell::IntPredicate::SLT,
                                ast::BinOp::LessThanOrEqual => inkwell::IntPredicate::SLE,
                                ast::BinOp::GreaterThan => inkwell::IntPredicate::SGT,
                                ast::BinOp::GreaterThanOrEqual => inkwell::IntPredicate::SGE,
                                _ => unreachable!(),
                            };
                            self.builder
                                .build_int_compare(predicate, lhs_int, rhs_int, "cmptmp")
                                .map_err(|e| anyhow::anyhow!("Failed to build comparison: {}", e))
                                .map(|v| v.into())
                        } else {
                            bail!("Comparison operation only supports integer values for now");
                        }
                    }
                    // Logical operators
                    ast::BinOp::And | ast::BinOp::Or => {
                        if !lhs.is_int_value() || !rhs.is_int_value() {
                            bail!("Logical operation only supports boolean values");
                        }

                        let lhs_int = lhs.into_int_value();
                        let rhs_int = rhs.into_int_value();

                        // Handle logical operations
                        match op {
                            ast::BinOp::And => self
                                .builder
                                .build_and(lhs_int, rhs_int, "andtmp")
                                .map_err(|e| anyhow::anyhow!("Failed to build AND: {}", e))
                                .map(|v| v.into()),
                            ast::BinOp::Or => self
                                .builder
                                .build_or(lhs_int, rhs_int, "ortmp")
                                .map_err(|e| anyhow::anyhow!("Failed to build OR: {}", e))
                                .map(|v| v.into()),
                            _ => unreachable!(),
                        }
                    }
                    // Arithmetic operators
                    _ => {
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
                            _ => unreachable!(),
                        }
                    }
                }
            }
            ast::Expr::UnaryOp { op, expr } => {
                let value = self.gen_expr(expr)?;

                match op {
                    ast::UnaryOp::Neg => {
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
                    ast::UnaryOp::Not => {
                        if !value.is_int_value() {
                            bail!("Logical NOT only supports boolean values");
                        }
                        let value = value.into_int_value();

                        self.builder
                            .build_not(value, "nottmp")
                            .map_err(|e| anyhow::anyhow!("Failed to build logical NOT: {}", e))
                            .map(|v| v.into())
                    }
                }
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
                let var_info = self
                    .env
                    .resolve_var(name)
                    .map_err(|e| anyhow::anyhow!("Variable '{}' not found: {}", name, e))?;
                // Load the value from the pointer
                self.builder
                    .build_load(var_info.ty, var_info.ptr, name) // Use stored type
                    .map_err(|e| anyhow::anyhow!("Failed to load variable '{}': {}", name, e))
            }
        }
    }

    /// Map AST type to LLVM type
    fn map_ast_type_to_llvm(&self, ty: ast::Type) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            ast::Type::I32 => Ok(self.context.i32_type().into()),
            ast::Type::I64 => Ok(self.context.i64_type().into()),
            ast::Type::F32 => Ok(self.context.f32_type().into()),
            ast::Type::F64 => Ok(self.context.f64_type().into()),
            ast::Type::Void => bail!("Void type cannot be used directly as a variable type"),
            ast::Type::String => bail!("String type not implemented"),
        }
    }

    /// Get default value for a given AST type
    fn get_default_value(&self, ty: ast::Type) -> Result<BasicValueEnum<'ctx>> {
        match ty {
            ast::Type::I32 => Ok(self.context.i32_type().const_zero().into()),
            ast::Type::I64 => Ok(self.context.i64_type().const_zero().into()),
            ast::Type::F32 => Ok(self.context.f32_type().const_zero().into()),
            ast::Type::F64 => Ok(self.context.f64_type().const_zero().into()),
            _ => bail!("Unsupported type for default value: {:?}", ty),
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

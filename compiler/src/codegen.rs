//! Visit the AST and generate the LLVM IR
//! Once the AST is built, the IR is generated (optionally optimized) and can be written to an
//! object file.

// TODO: Clean up the entire code, and make it idiomatic. This includes, but isn't limited to:
//     - Better documentation
//     - Add tests (after the parser is implemented?)
//     - Implement a better API?
use crate::ast::{
    BinaryOp::{self, *},
    Expression, FuncDef, Program, Statement, Type,
};
use crate::errors::*;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{FloatPredicate, IntPredicate, OptimizationLevel};
use std::collections::HashMap;
use std::path::Path;

pub struct CodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,

    symbol_table: HashMap<String, PointerValue<'ctx>>,
    current_function: Option<FunctionValue<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            module: context.create_module("main"),
            builder: context.create_builder(),
            symbol_table: HashMap::new(),
            current_function: None,
        }
    }

    /// Outputs the generated program to an object file. The function `gen_program` must have been
    /// called first. Optionally, the optimizer could also have been run.
    pub fn to_object_file(&self, obj_file_name: &str) {
        Target::initialize_all(&InitializationConfig::default());
        let triple = TargetMachine::get_default_triple();
        let target =
            Target::from_triple(&triple).expect("Couldn't create target from target triple");

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                OptimizationLevel::None,
                RelocMode::Default,
                CodeModel::Default,
            )
            .expect("Unable to create target machine");
        target_machine
            .write_to_file(&self.module, FileType::Object, Path::new(obj_file_name))
            .expect("Unable to write module to file");
    }

    /// Optimizes the program at the specified level (e.g. all optimizations are turned on in
    /// aggressive mode).
    pub fn optimize(&self, opt_level: OptimizationLevel) {
        let pass_manager_builder = PassManagerBuilder::create();
        pass_manager_builder.set_optimization_level(opt_level);

        let pass_manager = PassManager::create(());
        pass_manager_builder.populate_module_pass_manager(&pass_manager);
        pass_manager.run_on(&self.module);
    }

    /// Loops through all functions and generates their code
    pub fn gen_program(&mut self, program: &Program) -> Result<(), &'static str> {
        for func in &program.functions {
            self.gen_function(func)?;
        }
        Ok(())
    }

    fn gen_function(&mut self, func: &FuncDef) -> Result<(), &'static str> {
        let FuncDef {
            name,
            params,
            return_type,
            code: block,
        } = func;
        // The function parameter types
        let llvm_params = params
            .iter()
            .map(|(_, x)| x.to_llvm(self.context).into())
            .collect::<Vec<_>>();
        // The signature the function in LLVM terms
        let llvm_fn_sig = match return_type {
            Some(x) => x.to_llvm(self.context).fn_type(&llvm_params, false),
            None => self.context.void_type().fn_type(&llvm_params, false),
        };
        let function = self.module.add_function(name, llvm_fn_sig, None);
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.current_function = Some(function);
        // Set param names, an generate alloca and store instructions for them
        for (param, (param_name, param_type)) in function.get_param_iter().zip(params) {
            param.set_name(param_name);
            let alloca = self
                .builder
                .build_alloca(param_type.to_llvm(self.context), name);
            self.builder.build_store(alloca, param);
            self.symbol_table.insert(param_name.clone(), alloca);
        }
        self.gen_block(block)?;
        // We've got to return something, even if the function doesn't return
        if return_type.is_none() {
            self.builder.build_return(None);
        }
        Ok(())
    }

    fn gen_block(&mut self, block: &[Statement]) -> Result<(), &'static str> {
        for stmt in block {
            self.gen_statement(stmt)?
        }
        Ok(())
    }

    fn gen_statement(&mut self, stmt: &Statement) -> Result<(), &'static str> {
        match stmt {
            Statement::Assignment {
                name,
                var_type,
                expr,
            } => {
                let rhs = self.gen_expr(expr)?;
                let alloca = self
                    .builder
                    .build_alloca(var_type.to_llvm(self.context), name);
                self.builder.build_store(alloca, rhs);
                self.symbol_table.insert(name.clone(), alloca);
            }
            Statement::Return { expr } => {
                self.builder.build_return(Some(&self.gen_expr(expr)?));
            }
            Statement::Expression { expr } => {
                self.gen_expr(expr)?;
            }
            Statement::If {
                cond,
                then_block,
                else_block,
            } => self.gen_if(cond, then_block, else_block)?,
        };
        Ok(())
    }

    fn gen_expr(&self, expr: &Expression) -> Result<BasicValueEnum, &'static str> {
        match expr {
            Expression::Literal { expr_type, value } => Ok(self.gen_literal(expr_type, value)?),
            Expression::BinaryOp {
                op, left, right, ..
            } => Ok(self.gen_binop(op, left, right)?),
            Expression::Name { name, .. } => Ok(self.gen_var_ref(name)?),
            Expression::Call { func, args, .. } => Ok(self.gen_call(func, args)?),
        }
    }

    fn gen_var_ref(&self, name: &String) -> Result<BasicValueEnum, &'static str> {
        match self.symbol_table.get(name) {
            Some(var) => Ok(self.builder.build_load(*var, name)),
            None => {
                Err("reference to undefined variable (should have been caught by semantic checker)")
            }
        }
    }

    fn gen_literal(&self, expr_type: &Type, value: &str) -> Result<BasicValueEnum, &'static str> {
        match expr_type {
            Type::Int => Ok(BasicValueEnum::IntValue(
                self.context
                    .i64_type()
                    .const_int(value.parse::<i64>().unwrap() as u64, true),
            )),
            Type::Float32 => Ok(BasicValueEnum::FloatValue(
                self.context
                    .f32_type()
                    .const_float(value.parse::<f32>().unwrap().into()),
            )),
            Type::Float64 => Ok(BasicValueEnum::FloatValue(
                self.context
                    .f64_type()
                    .const_float(value.parse::<f64>().unwrap()),
            )),
            Type::Bool => Ok(BasicValueEnum::IntValue(
                self.context
                    .bool_type()
                    .const_int(value.parse::<u64>().unwrap(), true),
            )),
            Type::GoString => Ok(self
                .builder
                .build_global_string_ptr(&value.replace("\\n", "\n"), "str")
                .as_basic_value_enum()),
        }
    }

    fn gen_binop(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Result<BasicValueEnum, &'static str> {
        let left_gen = self.gen_expr(left)?;
        let right_gen = self.gen_expr(right)?;
        match (left_gen, right_gen) {
            // Binary operation of two ints
            (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                Ok(BasicValueEnum::IntValue(match op {
                    Add => self.builder.build_int_add(lhs, rhs, "addtmp"),
                    Sub => self.builder.build_int_sub(lhs, rhs, "subtmp"),
                    Mul => self.builder.build_int_mul(lhs, rhs, "multmp"),
                    Div => {
                        // Check if we are dividing by zero (results in undefined behavior)
                        let is_not_div_by_zero = self.builder.build_int_compare(
                            IntPredicate::NE,
                            rhs,
                            self.context.i64_type().const_int(0, true),
                            "is_not_div_by_zero"
                        );
                        let parent_bb = self.current_function.unwrap();
                        let panic_bb = self.context.append_basic_block(parent_bb, "panic_bb");
                        let cont_bb = self.context.append_basic_block(parent_bb, "cont_bb");
                        self.builder.build_conditional_branch(is_not_div_by_zero, cont_bb, panic_bb);

                        // panic_bb basic block
                        self.builder.position_at_end(panic_bb);
                        let error_msg = self.builder
                            .build_global_string_ptr(ERR_DIV_BY_ZERO, "div_by_zero")
                            .as_basic_value_enum();
                        self.builder.build_call(
                            self.module.get_function("__gopanic").unwrap(),
                            &[error_msg.into()],
                            "panic"
                        );
                        // Terminator instruction
                        self.builder.build_unreachable();

                        // If all is fine, continue at cont_bb
                        self.builder.position_at_end(cont_bb);
                        self.builder.build_int_signed_div(lhs, rhs, "divtmp")
                    },
                    Eq => self.builder.build_int_compare(IntPredicate::EQ, lhs, rhs, "eqtmp"),
                    Neq => self.builder.build_int_compare(IntPredicate::NE, lhs, rhs, "neqtmp"),
                    Ge => {
                        self.builder
                            .build_int_compare(IntPredicate::SGT, lhs, rhs, "getmp")
                    }
                    Le => {
                        self.builder
                            .build_int_compare(IntPredicate::SLT, lhs, rhs, "letmp")
                    }
                    Geq => self.builder.build_int_compare(
                        IntPredicate::SGE,
                        lhs,
                        rhs,
                        "geqtmp",
                    ),
                    Leq => self.builder.build_int_compare(
                        IntPredicate::SLE,
                        lhs,
                        rhs,
                        "leqtmp",
                    ),
                }))
            }
            // Binary operation of two floats (of same size)
            (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                if left.get_type() != right.get_type() {
                    return Err(
                        "cannot perform binary operation on float32 and float64 (should have been caught by the type checker)"
                    );
                }
                Ok(match op {
                    Add => BasicValueEnum::FloatValue(
                        self.builder.build_float_add(lhs, rhs, "addtmp"),
                    ),
                    Sub => BasicValueEnum::FloatValue(
                        self.builder.build_float_sub(lhs, rhs, "subtmp"),
                    ),
                    Mul => BasicValueEnum::FloatValue(
                        self.builder.build_float_mul(lhs, rhs, "multmp"),
                    ),
                    Div => BasicValueEnum::FloatValue(
                        self.builder.build_float_div(lhs, rhs, "divtmp"),
                    ),
                    Eq => {
                        BasicValueEnum::IntValue(self.builder.build_float_compare(
                            FloatPredicate::OEQ,
                            lhs,
                            rhs,
                            "eqtmp",
                        ))
                    }
                    Neq => {
                        BasicValueEnum::IntValue(self.builder.build_float_compare(
                            FloatPredicate::ONE,
                            lhs,
                            rhs,
                            "neqtmp",
                        ))
                    }
                    Ge => {
                        BasicValueEnum::IntValue(self.builder.build_float_compare(
                            FloatPredicate::OGT,
                            lhs,
                            rhs,
                            "getmp",
                        ))
                    }
                    Le => {
                        BasicValueEnum::IntValue(self.builder.build_float_compare(
                            FloatPredicate::OLT,
                            lhs,
                            rhs,
                            "letmp",
                        ))
                    }
                    Geq => {
                        BasicValueEnum::IntValue(self.builder.build_float_compare(
                            FloatPredicate::OGE,
                            lhs,
                            rhs,
                            "geqtmp",
                        ))
                    }
                    Leq => {
                        BasicValueEnum::IntValue(self.builder.build_float_compare(
                            FloatPredicate::OLE,
                            lhs,
                            rhs,
                            "leqtmp",
                        ))
                    }
                })
            }
            _ => Err("binary operations on unsupported types (should have been caught by the type checker)"),
        }
    }

    fn gen_call(&self, func: &String, args: &[Expression]) -> Result<BasicValueEnum, &'static str> {
        match self.module.get_function(func) {
            Some(func_value) => {
                let mut compiled_args = vec![];
                for arg in args {
                    compiled_args.push(self.gen_expr(arg)?.into());
                }
                match self
                    .builder
                    .build_call(func_value, compiled_args.as_slice(), "calltmp")
                    .try_as_basic_value()
                    .left()
                {
                    Some(value) => Ok(value),
                    // Because we got to return something from gen_expr, we return the
                    // magic number; It isn't used, so nothing lost there
                    None => Ok(BasicValueEnum::IntValue(
                        self.context.bool_type().const_int(1, true),
                    )),
                }
            }
            None => Err("undefined function passed to codegen (should have been caught by semantic checker)"),
        }
    }

    fn gen_if(
        &mut self,
        cond: &Expression,
        then_block: &[Statement],
        else_block: &[Statement],
    ) -> Result<(), &'static str> {
        let parent = self.current_function.unwrap();

        let llvm_cond = self.gen_expr(cond)?.into_int_value();

        let then_bb = self.context.append_basic_block(parent, "then_bb");
        let else_bb = self.context.append_basic_block(parent, "else_bb");
        let cont_bb = self.context.append_basic_block(parent, "cont_bb");

        self.builder
            .build_conditional_branch(llvm_cond, then_bb, else_bb);

        // Then block
        self.builder.position_at_end(then_bb);
        self.gen_block(then_block)?;
        self.builder.build_unconditional_branch(cont_bb);

        // Else block
        self.builder.position_at_end(else_bb);
        self.gen_block(else_block)?;
        self.builder.build_unconditional_branch(cont_bb);

        // Merge/continuation block
        self.builder.position_at_end(cont_bb);
        Ok(())
    }
}

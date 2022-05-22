//! Visit the AST and generate the LLVM IR
//! Once the AST is built, the IR is generated (optionally optimized)
//! and can be written to an object file.

use crate::ast::{
    BinaryOp::{self, *},
    Expression, FuncDef, Program, Statement, Type,
};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::{FloatPredicate, IntPredicate, OptimizationLevel};
use std::collections::HashMap;
use std::path::Path;

pub struct CodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,

    pub symbol_table: HashMap<String, PointerValue<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn to_object_file(&self, obj_file_name: &str) {
        Target::initialize_all(&InitializationConfig::default());
        let triple = TargetMachine::get_default_triple();
        let target =
            Target::from_triple(&triple).expect("couldn't create target from target triple");

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                OptimizationLevel::None,
                RelocMode::Default,
                CodeModel::Default,
            )
            .expect("unable to create target machine");
        target_machine
            .write_to_file(&self.module, FileType::Object, Path::new(obj_file_name))
            .expect("unable to write module to file");
    }

    pub fn optimize(&self, opt_level: OptimizationLevel) {
        let pass_manager_builder = PassManagerBuilder::create();
        pass_manager_builder.set_optimization_level(opt_level);

        let pass_manager = PassManager::create(());
        pass_manager_builder.populate_module_pass_manager(&pass_manager);
        pass_manager.run_on(&self.module);
    }

    pub fn gen_program(&mut self, program: &Program) {
        for func in &program.functions {
            self.gen_function(func);
        }
    }

    fn gen_function(&mut self, func: &FuncDef) {
        let FuncDef {
            name,
            params,
            return_type,
            code,
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
        // Set param names, an generate alloca and store instructions for them
        for (param, (param_name, param_type)) in function.get_param_iter().zip(params) {
            param.set_name(param_name);
            let alloca = self
                .builder
                .build_alloca(param_type.to_llvm(self.context), name);
            self.builder.build_store(alloca, param);
            self.symbol_table.insert(param_name.clone(), alloca);
        }
        // Generate function body
        for stmt in code {
            self.gen_statement(stmt);
        }
        if return_type.is_none() {
            self.builder.build_return(None);
        }
    }

    fn gen_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assignment {
                name,
                var_type,
                expr,
            } => {
                let rhs = self.gen_expr(expr);
                let alloca = self
                    .builder
                    .build_alloca(var_type.to_llvm(self.context), name);
                self.builder.build_store(alloca, rhs);
                self.symbol_table.insert(name.clone(), alloca);
            }
            Statement::Return { expr } => {
                self.builder.build_return(Some(&self.gen_expr(expr)));
            }
            Statement::Expression { expr } => {
                self.gen_expr(expr);
            }
            _ => panic!("REMOVE AFTER STMT IMPLEMENTED"),
        };
    }

    fn gen_expr(&self, expr: &Expression) -> BasicValueEnum {
        match expr {
            Expression::Literal { expr_type, value } => self.gen_literal(expr_type, value),
            Expression::BinaryOp {
                op, left, right, ..
            } => self.gen_binop(op, left, right),
            Expression::Name { name, .. } => self.gen_var_ref(name),
            Expression::Call { func, args, .. } => self.gen_call(func, args),
        }
    }

    fn gen_var_ref(&self, name: &String) -> BasicValueEnum {
        match self.symbol_table.get(name) {
            Some(var) => self.builder.build_load(*var, name),
            None => panic!(
                "reference to undefined variable (should have been caught by semantic checker)"
            ),
        }
    }

    fn gen_literal(&self, expr_type: &Type, value: &str) -> BasicValueEnum {
        match expr_type {
            Type::Int => BasicValueEnum::IntValue(
                self.context
                    .i64_type()
                    .const_int(value.parse::<i64>().unwrap() as u64, true),
            ),
            Type::Float32 => BasicValueEnum::FloatValue(
                self.context
                    .f32_type()
                    .const_float(value.parse::<f32>().unwrap().into()),
            ),
            Type::Float64 => BasicValueEnum::FloatValue(
                self.context
                    .f64_type()
                    .const_float(value.parse::<f64>().unwrap()),
            ),
            Type::Bool => BasicValueEnum::IntValue(
                self.context
                    .bool_type()
                    .const_int(value.parse::<u64>().unwrap(), true),
            ),
            Type::GoString => self
                .builder
                .build_global_string_ptr(value, "str")
                .as_basic_value_enum(),
        }
    }

    fn gen_binop(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> BasicValueEnum {
        let left_gen = self.gen_expr(left);
        let right_gen = self.gen_expr(right);
        match (left_gen, right_gen) {
            // Binary operation of two ints
            (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                BasicValueEnum::IntValue(match op {
                    Add => self.builder.build_int_add(lhs, rhs, "addtmp"),
                    Sub => self.builder.build_int_sub(lhs, rhs, "subtmp"),
                    Mul => self.builder.build_int_mul(lhs, rhs, "multmp"),
                    // TODO: Add div by zero check (sdiv results in undefined behavior in this case)
                    Div => self.builder.build_int_signed_div(lhs, rhs, "divtmp"),
                    Eq => {
                        self.builder
                            .build_int_compare(IntPredicate::EQ, lhs, rhs, "eqtmp")
                    }
                    Neq => {
                        self.builder
                            .build_int_compare(IntPredicate::NE, lhs, rhs, "neqtmp")
                    }
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
                })
            }
            // Binary operation of two floats (of same size)
            (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                assert!(
                    left.get_type() == right.get_type(),
                    "cannot add float32 and float64 (should have been caught by the type checker)"
                );
                match op {
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
                }
            }
            _ => panic!(
                "binary operations on unsupported types (should have been caught by the type checker)"
            ),
        }
    }

    fn gen_call(&self, func: &String, args: &[Expression]) -> BasicValueEnum {
        match self.module.get_function(func) {
            Some(func_value) => {
                let compiled_args = args
                    .iter()
                    .map(|x| self.gen_expr(x).into())
                    .collect::<Vec<_>>();
                match self
                    .builder
                    .build_call(func_value, compiled_args.as_slice(), "calltmp")
                    .try_as_basic_value()
                    .left()
                {
                    Some(value) => value,
                    // Because we got to return something from gen_expr, we return the
                    // magic number; It isn't used, so nothing lost there
                    None => BasicValueEnum::IntValue(self.context.bool_type().const_int(1, true)),
                }
            }
            None => panic!(
                "function {} not defined (should have been caught by semantic checker)",
                func
            ),
        }
    }
}

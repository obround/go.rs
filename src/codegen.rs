// Visit the AST and generate the LLVM IR

use crate::ast::{BinaryOp::*, Expression, FuncDef, Program, Statement, Type};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Module;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};
use std::collections::HashMap;

pub struct CodeGen<'a, 'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub execution_engine: ExecutionEngine<'ctx>,
    pub program: &'a Program,

    pub symbol_table: HashMap<String, PointerValue<'ctx>>,
}

impl<'a, 'ctx> CodeGen<'a, 'ctx> {
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
            .collect::<Vec<BasicMetadataTypeEnum>>();
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
            param.set_name(&param_name);
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
            },
            _ => panic!("REMOVE AFTER STMT IMPLEMENTED"),
        };
    }
    fn gen_expr(&self, expr: &Expression) -> BasicValueEnum {
        match expr {
            Expression::Literal { expr_type, value } => match expr_type {
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
            },
            Expression::BinaryOp {
                op, left, right, ..
            } => {
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
            Expression::Name { name, .. } => match self.symbol_table.get(name) {
                Some(var) => self.builder.build_load(*var, name),
                None => panic!(
                    "reference to undefined variable (should have been caught by semantic checker)"
                ),
            },
        }
    }
}

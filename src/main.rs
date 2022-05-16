// llvmgo playground

mod ast;
mod codegen;
mod pretty_printer;
use crate::codegen::CodeGen;
use ast::*;
use inkwell::{context::Context, OptimizationLevel};
use std::collections::HashMap;

fn main() {
    let program = Program {
        package_name: "main".to_string(),
        imports: vec![],
        functions: vec![FuncDef {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            code: vec![
                Statement::Assignment {
                    name: "x".to_string(),
                    var_type: Type::Int,
                    expr: Expression::Literal {
                        expr_type: Type::Int,
                        value: "2".to_string(),
                    },
                },
                Statement::Assignment {
                    name: "y".to_string(),
                    var_type: Type::Int,
                    expr: Expression::BinaryOp {
                        expr_type: Type::Int,
                        op: BinaryOp::Mul,
                        left: Box::new(Expression::Name {
                            expr_type: Type::Int,
                            name: "x".to_string(),
                        }),
                        right: Box::new(Expression::Literal {
                            expr_type: Type::Int,
                            value: "2".to_string(),
                        }),
                    },
                },
            ],
        }],
    };
    let context = Context::create();
    let module = context.create_module("main");
    let execution_engine = module
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();
    let mut codegen = CodeGen {
        context: &context,
        module,
        builder: context.create_builder(),
        execution_engine,
        program: &program,
        symbol_table: (HashMap::new()),
    };
    codegen.gen_program(&program);
    println!("{}", codegen.module.print_to_string().to_string());
    // println!("{}", pretty_printer::format_program(&program));
}

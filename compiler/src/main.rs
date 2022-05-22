use compiler::{ast::*, compile_aot, pretty_printer::format_program};

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
                    expr: Expression::Call {
                        expr_type: Some(Type::Int),
                        func: "add".to_string(),
                        args: vec![
                            Expression::Literal {
                                expr_type: Type::Int,
                                value: "2".to_string(),
                            },
                            Expression::Literal {
                                expr_type: Type::Int,
                                value: "3".to_string(),
                            },
                        ],
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
                Statement::Assignment {
                    name: "z".to_string(),
                    var_type: Type::GoString,
                    expr: Expression::Literal {
                        expr_type: Type::GoString,
                        value: "hello world!".to_string(),
                    },
                },
                Statement::Expression {
                    expr: Expression::Call {
                        expr_type: None,
                        func: "print_str".to_string(),
                        args: vec![Expression::Name {
                            expr_type: Type::GoString,
                            name: "z".to_string(),
                        }],
                    },
                },
            ],
        }],
    };
    println!("------- GO CODE: -------");
    println!("{}", format_program(&program));
    println!("------- LLVM IR: -------");
    println!("{}", compile_aot(&program, "output/main"));
}

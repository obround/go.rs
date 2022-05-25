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
                    expr: Expression::Literal {
                        expr_type: Type::Int,
                        value: "10".to_string(),
                    },
                },
                Statement::Assignment {
                    name: "y".to_string(),
                    var_type: Type::Int,
                    expr: Expression::BinaryOp {
                        expr_type: Type::Int,
                        op: BinaryOp::Div,
                        left: Box::new(Expression::Literal {
                            expr_type: Type::Int,
                            value: "5".to_string(),
                        }),
                        right: Box::new(Expression::Name {
                            expr_type: Type::Int,
                            name: "x".to_string(),
                        }),
                    },
                },
                Statement::Assignment {
                    name: "lit".to_string(),
                    var_type: Type::GoString,
                    expr: Expression::Literal {
                        expr_type: Type::GoString,
                        value: "hello world! my number is ".to_string(),
                    },
                },
                Statement::Expression {
                    expr: Expression::Call {
                        expr_type: None,
                        func: "__print_gostring".to_string(),
                        args: vec![Expression::Name {
                            expr_type: Type::GoString,
                            name: "lit".to_string(),
                        }],
                    },
                },
                Statement::Expression {
                    expr: Expression::Call {
                        expr_type: None,
                        func: "__print_int".to_string(),
                        args: vec![Expression::Name {
                            expr_type: Type::GoString,
                            name: "y".to_string(),
                        }],
                    },
                },
                Statement::Expression {
                    expr: Expression::Call {
                        expr_type: None,
                        func: "__print_gostring".to_string(),
                        args: vec![Expression::Literal {
                            expr_type: Type::GoString,
                            value: "\\n".to_string(),
                        }],
                    },
                },
                Statement::If {
                    cond: Expression::BinaryOp {
                        expr_type: Type::Bool,
                        op: BinaryOp::Eq,
                        left: Box::new(Expression::Literal {
                            expr_type: Type::Float32,
                            value: "5".to_string(),
                        }),
                        right: Box::new(Expression::Literal {
                            expr_type: Type::Float32,
                            value: "5".to_string(),
                        }),
                    },
                    then_block: vec![
                        Statement::Expression {
                            expr: Expression::Call {
                                expr_type: None,
                                func: "__print_gostring".to_string(),
                                args: vec![Expression::Literal {
                                    expr_type: Type::GoString,
                                    value: "good ".to_string(),
                                }],
                            },
                        },
                        Statement::Expression {
                            expr: Expression::Call {
                                expr_type: None,
                                func: "__print_gostring".to_string(),
                                args: vec![Expression::Literal {
                                    expr_type: Type::GoString,
                                    value: "✓\\n".to_string(),
                                }],
                            },
                        },
                    ],
                    else_block: vec![Statement::Expression {
                        expr: Expression::Call {
                            expr_type: None,
                            func: "__print_gostring".to_string(),
                            args: vec![Expression::Literal {
                                expr_type: Type::GoString,
                                value: "oops\\n".to_string(),
                            }],
                        },
                    }],
                },
                // Statement::Expression {
                //     expr: Expression::Call {
                //         expr_type: None,
                //         func: "__flush_stdout".to_string(),
                //         args: vec![],
                //     },
                // },
            ],
        }],
    };
    println!("------- GO CODE: -------");
    println!("{}", format_program(&program));
    println!("------- LLVM IR: -------");
    println!("{}", compile_aot(&program, "output/main"));
}

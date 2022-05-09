mod ast;
mod pretty_printer;
use ast::*;

fn main() {
    let code_ast = Program {
        package_name: "main".to_string(),
        imports: vec![],
        functions: vec![FuncDef {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            code: vec![
                Statement::Assignment(
                    "x".to_string(),
                    Type::Int,
                    Expression::Literal(Type::Int, "2".to_string()),
                ),
                Statement::Assignment(
                    "y".to_string(),
                    Type::Int,
                    Expression::BinaryOp(
                        BinaryOp::Mul,
                        Box::new(Expression::Name("x".to_string())),
                        Box::new(Expression::Literal(Type::Int, "2".to_string())),
                    ),
                ),
            ],
        }],
    };
    println!("{}", pretty_printer::format_program(&code_ast));
}

// Prints the AST as (well formatted) Go code

use crate::ast::*;

fn format_type(r#type: &Type) -> String {
    match r#type {
        Type::Int => "int",
        Type::Float32 => "float32",
        Type::Float64 => "float64",
    }
    .to_string()
}

fn format_name_type((name, r#type): &(String, Type)) -> String {
    format!("{} {}", name, format_type(r#type))
}

fn format_params(params: &Params) -> String {
    params
        .iter()
        .map(format_name_type)
        .collect::<Vec<String>>()
        .join(", ")
}

fn format_bop(bop: &BinaryOp) -> String {
    match bop {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Eq => "==",
        BinaryOp::Neq => "!=",
        BinaryOp::Ge => ">",
        BinaryOp::Le => "<",
        BinaryOp::Geq => ">=",
        BinaryOp::Leq => "<=",
    }
    .to_string()
}

fn push_indent(indent: usize, s: &mut String) {
    s.push_str(&" ".repeat(indent));
}

pub fn format_program(program: &Program) -> String {
    let mut s = format!("package {}\n\n", program.package_name);
    if program.imports.len() > 0 {
        s.push_str(&format!("import (\"{}\")\n\n", program.imports.join(", ")));
    }
    s.push_str(
        &program
            .functions
            .iter()
            .map(format_funcdef)
            .collect::<Vec<String>>()
            .join("\n"),
    );
    s
}

fn format_funcdef(funcdef: &FuncDef) -> String {
    let FuncDef {
        name,
        params,
        return_type,
        code,
    } = funcdef;
    let mut s = format!("func {}({}) ", name, format_params(&params));
    if let Some(r#type) = return_type {
        s.push_str(&(format_type(r#type) + &" "));
    }
    s.push_str(&format_code_block(code, 0));
    s
}

fn format_code_block(code: &Vec<Statement>, indent: usize) -> String {
    let mut s = "{\n".to_string();

    for statement in code {
        s.push_str(&format_statement(statement, indent + 4));
        s.push_str("\n");
    }

    push_indent(indent, &mut s);
    s.push_str("}\n");
    s
}

fn format_statement(statement: &Statement, indent: usize) -> String {
    " ".repeat(indent)
        + &match statement {
            Statement::Assignment(name, r#type, expr) => format!(
                "var {} {} = {}",
                name,
                format_type(r#type),
                format_expression(expr)
            ),
            Statement::If(cond, code) => format!(
                "if {} {}",
                format_expression(cond),
                format_code_block(code, indent + 4)
            ),
            Statement::Call(expr, args) => format!(
                "{}({})",
                format_expression(expr),
                args.iter()
                    .map(format_expression)
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Statement::Return(expr) => format!("return {}", format_expression(expr)),
        }
}

fn format_expression(expr: &Expression) -> String {
    match expr {
        Expression::True => "true".to_string(),
        Expression::False => "false".to_string(),
        Expression::Name(name) => name.clone(),
        Expression::Literal(_, value) => value.clone(),
        Expression::BinaryOp(bop, left, right) => format!(
            "{} {} {}",
            format_expression(left),
            format_bop(bop),
            format_expression(right)
        ),
    }
}

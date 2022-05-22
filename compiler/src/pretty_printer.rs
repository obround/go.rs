// Prints the AST as (well formatted) Go code

use crate::ast::*;

fn format_type(r#type: &Type) -> String {
    match r#type {
        Type::Int => "int",
        Type::Float32 => "float32",
        Type::Float64 => "float64",
        Type::Bool => "bool",
        Type::GoString => "string",
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
    if !program.imports.is_empty() {
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
    let mut s = format!("func {}({}) ", name, format_params(params));
    if let Some(r#type) = return_type {
        s.push_str(&(format_type(r#type) + " "));
    }
    s.push_str(&format_code_block(code, 0));
    s
}

fn format_code_block(code: &Vec<Statement>, indent: usize) -> String {
    let mut s = "{\n".to_string();

    for statement in code {
        s.push_str(&format_statement(statement, indent + 4));
        s.push('\n');
    }

    push_indent(indent, &mut s);
    s.push_str("}\n");
    s
}

fn format_statement(statement: &Statement, indent: usize) -> String {
    " ".repeat(indent)
        + &match statement {
            Statement::Assignment {
                name,
                var_type,
                expr,
            } => format!(
                "var {} {} = {}",
                name,
                format_type(var_type),
                format_expression(expr)
            ),
            Statement::If { cond, block } => format!(
                "if {} {}",
                format_expression(cond),
                format_code_block(block, indent + 4)
            ),
            Statement::Return { expr } => format!("return {}", format_expression(expr)),
            Statement::Expression { expr } => format_expression(expr),
        }
}

fn format_expression(expr: &Expression) -> String {
    match expr {
        Expression::Name { name, .. } => name.clone(),
        Expression::Literal { expr_type, value } => match expr_type {
            Type::Bool => (if value == "1" { "true" } else { "false" }).to_string(),
            Type::GoString => format!("\"{}\"", value),
            _ => value.clone(),
        },
        Expression::BinaryOp {
            op, left, right, ..
        } => format!(
            "{} {} {}",
            format_expression(left),
            format_bop(op),
            format_expression(right)
        ),
        Expression::Call { func, args, .. } => format!(
            "{}({})",
            func,
            args.iter()
                .map(format_expression)
                .collect::<Vec<String>>()
                .join(", ")
        ),
    }
}

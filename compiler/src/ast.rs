// Structs and enums that will store the output of the Go parser allowing for LLVM code generation

use inkwell::{context::Context, types::BasicTypeEnum, AddressSpace};

pub type CodeBlock = Vec<Statement>;
pub type Params = Vec<(String, Type)>;

#[derive(Debug)]
pub struct Program {
    pub package_name: String, // package package_name
    pub imports: Vec<String>, // import (mod_1, ..)
    pub functions: Vec<FuncDef>,
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub params: Params,            // (name1 type1, name2 type2, ..)
    pub return_type: Option<Type>, // returning is optional
    pub code: CodeBlock,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    Int,      // i64
    Bool,     // i1
    Float32,  // f32
    Float64,  // f64
    GoString, // string
}

#[derive(Debug)]
pub enum Expression {
    Name {
        expr_type: Type,
        name: String,
    },
    // NOTE: If the literal is a bool, then the value is 1 for true and 0 for false
    Literal {
        expr_type: Type,
        value: String,
    },
    BinaryOp {
        expr_type: Type,
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Call {
        expr_type: Option<Type>,
        func: String, // String for now; Later on maybe Box<Expression>
        args: Vec<Expression>,
    },
}

#[derive(Debug)]
pub enum BinaryOp {
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Eq,  // ==
    Neq, // !=
    Ge,  // >
    Le,  // <
    Geq, // >=
    Leq, // <=
}

#[derive(Debug)]
pub enum Statement {
    Assignment {
        name: String,
        var_type: Type,
        expr: Expression,
    },
    If {
        cond: Expression,
        block: CodeBlock,
    },
    Return {
        expr: Expression,
    },
    Expression {
        expr: Expression,
    },
}

impl Type {
    // Convert to an LLVM type
    pub fn to_llvm<'ctx>(&self, context: &'ctx Context) -> BasicTypeEnum<'ctx> {
        match self {
            Type::Int => BasicTypeEnum::IntType(context.i64_type()),
            Type::Float32 => BasicTypeEnum::FloatType(context.f32_type()),
            Type::Float64 => BasicTypeEnum::FloatType(context.f64_type()),
            Type::Bool => BasicTypeEnum::IntType(context.bool_type()),
            Type::GoString => {
                BasicTypeEnum::PointerType(context.i8_type().ptr_type(AddressSpace::Generic))
            }
        }
    }

    pub fn get_num_precision(&self) -> usize {
        match self {
            Type::Int => 64,
            Type::Float32 => 32,
            Type::Float64 => 64,
            Type::Bool => 1,
            _ => panic!("get_num_precision is not supported for string"),
        }
    }
}

impl Expression {
    pub fn get_type(&self) -> &Type {
        match self {
            Expression::Name { expr_type, .. } => expr_type,
            Expression::Literal { expr_type, .. } => expr_type,
            Expression::BinaryOp { expr_type, .. } => expr_type,
            Expression::Call { expr_type, .. } => expr_type
                .as_ref()
                .expect("Expression::get_type() should not be called on a void function"),
        }
    }
}

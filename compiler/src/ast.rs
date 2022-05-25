//! Structs and enums that will store the output of the Go parser, facilitating LLVM code
//! generation.

use inkwell::{context::Context, types::BasicTypeEnum, AddressSpace};

/// A block of code (which is a vector of statements)
pub type CodeBlock = Vec<Statement>;
/// Of the form `[(name, type), ...]`
pub type Params = Vec<(String, Type)>;

/// The top-level go package.
#[derive(Debug)]
pub struct Program {
    /// `package package_name`
    pub package_name: String,
    /// `import (mod_1, ..)`
    pub imports: Vec<String>,
    pub functions: Vec<FuncDef>,
}

/// A function in the go package. If `return_value` is `None`, then
/// the function is of type `void`.
#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    /// `(name1 type1, name2 type2, ..)`
    pub params: Params,
    pub return_type: Option<Type>,
    pub code: CodeBlock,
}

/// Currently, only some go types are supported:
/// * `go_type` (`llvm_type`)
/// * `int` (`i64`)
/// * `bool` (`i1`)
/// * `float32` (`f32`)
/// * `float64` (`f64`)
/// * `string` (`i8*`)
#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    Float32,
    Float64,
    GoString,
}

#[derive(Debug)]
pub enum Expression {
    Name {
        expr_type: Type,
        name: String,
    },
    /// Something like an integer of float. Note that if the literal is a bool,
    /// then the value is either 0 for false, or 1 for true
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
        /// Currently you only call a function, not an expression that
        /// evaluates to a function (e.g. closure, methods, etc.)
        func: String,
        args: Vec<Expression>,
    },
}

#[derive(Debug)]
pub enum BinaryOp {
    /// +
    Add,
    /// \-
    Sub,
    /// \*
    Mul,
    /// /
    Div,
    /// ==
    Eq,
    /// !=
    Neq,
    /// \>
    Ge,
    /// \<
    Le,
    /// \>=
    Geq,
    /// \<=
    Leq,
}

#[derive(Debug)]
pub enum Statement {
    /// `var <name> <var_type> = <expr>`
    Assignment {
        name: String,
        var_type: Type,
        expr: Expression,
    },
    /// `if <cond> { <then_block> } else { <else_block2> }`
    If { cond: Expression, then_block: CodeBlock, else_block: CodeBlock },
    /// `return <expr>`
    Return { expr: Expression },
    /// `<expr>`
    Expression { expr: Expression },
}

impl Type {
    /// Convert to an LLVM type. Very useful during code generation
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
}

impl Expression {
    /// Returns the type the expression is tagged with
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

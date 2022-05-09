// Types that will store the output of the Go parser allowing for LLVM code generation
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

#[derive(Debug)]
pub enum Type {
    Int,     // 64 bit int
    Float32, // 32 bit float
    Float64, // 64 bit float
}

#[derive(Debug)]
pub enum Expression {
    True,
    False,
    Name(String),
    Literal(Type, String),                                // type(value)
    BinaryOp(BinaryOp, Box<Expression>, Box<Expression>), // expr1 op expr2
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
    Assignment(String, Type, Expression), // var name type = expr
    If(Expression, CodeBlock),            // if expr { <codeblock> }
    Call(Expression, Vec<Expression>),    // expr(arg1, ..)
    Return(Expression),
}

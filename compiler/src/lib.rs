//! Ties all parts of the compiler together

pub mod ast;
pub mod codegen;
pub mod errors;
pub mod pretty_printer;
use ast::*;
use codegen::CodeGen;
use inkwell::module::Module;
use inkwell::{context::Context, module::Linkage};
use inkwell::{AddressSpace, OptimizationLevel};
use std::fs;
use std::process::Command;

// TODO: Clean this steaming piece of garbage up. This includes, but isn't limited to:
//     - Handle debug and release filepaths while linking with runtime
//     - For the love of god, please handle stderr while compiling and linking runtime
//     - Add a nice command line interface

/// Adds an extern (runtime) function to the module. All the types must be `context.<my_type>()`.
/// Here is the format of the function:
///
/// `add_runtime_func(module, "func_name", return_type, [arg1, arg2, ...])`
macro_rules! add_runtime_func {
    ($module:ident, $func_name:literal, $ret_type:expr, [ $($arg_type:expr),* $(,)? ] $(,)?) => {
        $module.add_function(
            $func_name,
            $ret_type.fn_type(
                &[$( $arg_type.into() ),*],
                false,
            ),
            Some(Linkage::External),
        )
    };
}

fn add_runtime<'a>(module: &Module<'a>, context: &'a Context) {
    add_runtime_func!(module, "__flush_stdout", context.void_type(), []);
    add_runtime_func!(
        module,
        "__gopanic",
        context.void_type(),
        [context.i8_type().ptr_type(AddressSpace::Generic)],
    );
    add_runtime_func!(
        module,
        "add",
        context.i64_type(),
        [context.i64_type(), context.i64_type()],
    );
    add_runtime_func!(
        module,
        "__print_int",
        context.void_type(),
        [context.i64_type()],
    );
    add_runtime_func!(
        module,
        "__print_bool",
        context.void_type(),
        [context.bool_type()],
    );
    add_runtime_func!(
        module,
        "__print_float32",
        context.void_type(),
        [context.f32_type()],
    );
    add_runtime_func!(
        module,
        "__print_float64",
        context.void_type(),
        [context.f64_type()],
    );
    add_runtime_func!(
        module,
        "__print_gostring",
        context.void_type(),
        [context.i8_type().ptr_type(AddressSpace::Generic)],
    );
}

// NOTE: Once the parser is implemented, `program` will be a &str for file path
pub fn compile_aot(program: &Program, out_path: &str) -> String {
    let context = Context::create();
    // Add global (external) decelerations
    let mut codegen = CodeGen::new(&context);
    add_runtime(&codegen.module, &context);
    if let Err(err) = codegen.gen_program(program) {
        panic!("{}", err);
    }
    codegen.optimize(OptimizationLevel::Aggressive);

    // Create directory `output` if it doesn't already exist
    fs::create_dir_all("output/").expect("unable to create output/");
    // Generate object file
    codegen.to_object_file(&format!("output/{}.o", program.package_name));
    // Compile runtime to object file
    let compile_runtime = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--manifest-path",
            "./runtime/Cargo.toml",
        ])
        .output();
    match compile_runtime {
        Ok(_) => {}
        Err(err) => panic!("{}", err),
    }
    // Link runtime and package`
    let link_runtime = Command::new("gcc")
        .args(&[
            "-flto",
            "-dead_strip",
            "runtime/target/release/libruntime.a",
            &format!("output/{}.o", program.package_name),
            "-o",
            out_path,
        ])
        .output();
    match link_runtime {
        Ok(result) => {
            if !result.stderr.is_empty() {
                panic!("{}", std::str::from_utf8(&result.stderr).unwrap().trim());
            }
        }
        Err(err) => panic!("{}", err),
    }
    codegen.module.print_to_string().to_string() // Return LLVM IR
}

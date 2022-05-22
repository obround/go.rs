pub mod ast;
pub mod codegen;
pub mod pretty_printer;
use ast::*;
use codegen::CodeGen;
use inkwell::module::Module;
use inkwell::{context::Context, module::Linkage};
use inkwell::{AddressSpace, OptimizationLevel};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

fn add_runtime<'a>(module: &Module<'a>, context: &'a Context) {
    module.add_function(
        "add",
        context.i64_type().fn_type(
            &[context.i64_type().into(), context.i64_type().into()],
            false,
        ),
        Some(Linkage::External),
    );
    module.add_function(
        "print_str",
        context.void_type().fn_type(
            &[context.i8_type().ptr_type(AddressSpace::Generic).into()],
            false,
        ),
        Some(Linkage::External),
    );
}

// NOTE: Once the parser is implemented, `program` will be a &str for file path
pub fn compile_aot(program: &Program, out_path: &str) -> String {
    let context = Context::create();
    let module = context.create_module("main");
    // Add global (external) decelerations
    add_runtime(&module, &context);
    let mut codegen = CodeGen {
        context: &context,
        module,
        builder: context.create_builder(),
        symbol_table: (HashMap::new()),
    };
    codegen.gen_program(program);
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
    // TODO: Handle stderr when compiling and linking runtime
    match compile_runtime {
        Ok(_) => {}
        Err(err) => panic!("{}", err),
    }
    // TODO: Handle Debug and Release filepaths
    // Link runtime and package
    let link_runtime = Command::new("clang")
        .args(&[
            "-flto",
            "target/debug/libruntime.a",
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

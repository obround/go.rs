//! Runtime functions for the generated binaries. The functions defined here are 'extern'ed, and
//! later linked with the code.

// TODO: Handle possible runtime panics that *rust* might invoke. E.g., the print! macro will panic
// if it can't write to io::stdout(). Make sure that these error messages follow go's conventions.

extern crate libc;

use libc::c_char;
use std::{
    ffi::CStr,
    io::{self, Write},
    process,
};

macro_rules! __local_go_panic {
    ($msg:expr) => {{
        eprintln!("panic: {}", $msg);
        process::abort();
    }};
}

macro_rules! cstr_to_str {
    ($msg:expr) => {
        match CStr::from_ptr($msg).to_str() {
            Ok(cstr) => cstr,
            Err(_) => __local_go_panic!("unable to interpret passed string"),
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn __gopanic(msg: *const c_char) {
    __local_go_panic!(cstr_to_str!(msg));
}

#[no_mangle]
pub extern "C" fn __flush_stdout() {
    if io::stdout().flush().is_err() {
        __local_go_panic!("unable to flush to stdout");
    }
}

#[no_mangle]
pub unsafe extern "C" fn __print_int(int: i64) {
    print!("{}", int);
}

#[no_mangle]
pub unsafe extern "C" fn __print_bool(boolean: bool) {
    print!("{}", boolean);
}

#[no_mangle]
pub unsafe extern "C" fn __print_float32(float: f32) {
    print!("{}", float);
}

#[no_mangle]
pub unsafe extern "C" fn __print_float64(float: f64) {
    print!("{}", float);
}

#[no_mangle]
pub unsafe extern "C" fn __print_gostring(string: *const c_char) {
    print!("{}", cstr_to_str!(string));
}

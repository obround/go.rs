//! Runtime functions for go.rs.

extern crate libc;

use libc::c_char;
use std::{
    ffi::CStr,
    io::{self, Write},
    process,
};

#[inline(always)]
fn __local_go_panic(msg: &str) {
    eprintln!("panic: {}", msg);
    process::abort();
}

#[no_mangle]
pub unsafe extern "C" fn __gopanic(msg: *const c_char) {
    match CStr::from_ptr(msg).to_str() {
        Ok(cstr) => __local_go_panic(cstr),
        Err(_) => __local_go_panic("unable to print custom panic (due to incorrect string)"),
    };
}

#[no_mangle]
pub extern "C" fn __flush_stdout() {
    if io::stdout().flush().is_err() {
        __local_go_panic("unable to flush to stdout");
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
    match CStr::from_ptr(string).to_str() {
        Ok(cstr) => print!("{}", cstr),
        Err(_) => __local_go_panic("unable to print (due to incorrect string)"),
    };
}

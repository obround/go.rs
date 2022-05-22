// Runtime functions for go.rs

extern crate libc;

use libc::c_char;
use std::ffi::CStr;

#[no_mangle]
extern "C" fn add(x: i64, y: i64) -> i64 {
    x + y
}

#[no_mangle]
pub unsafe extern "C" fn print_str(string: *const c_char) {
    println!("{}", CStr::from_ptr(string).to_str().unwrap());
}

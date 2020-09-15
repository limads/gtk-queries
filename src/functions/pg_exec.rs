/*use libloading::{Library, Symbol};
use crate::tables::{table::*, column::*};
use std::convert::{TryFrom, TryInto};
use super::loader::FunctionErr;
use super::sql_type::*;
use rusqlite::types::{ToSql, FromSql};
use rusqlite::functions::FunctionFlags;
use super::function::Function;
use std::any::Any;
use std::mem;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::ffi::CString;

// To generate the C header: cbindgen src/functions/pg_exec.rs -o /home/diego/Downloads/pg_exec.h --lang c
#[repr(C)]
pub union PGType {
    I64 : i64,
    F64 : f64,
    Text : *const u8,
    Bytes : *const PGBytes
}

/*impl From<String> for PGType {

    fn from(s : String) -> PGType {
        unsafe {
            let s = CString::new(s.as_bytes()).unwrap();
            mem::forget(s);
            PGType{ Text : s.into_raw() }
        }
    }
}*/

/*impl From<Vec<u8>> for PGType {

}*/

impl From<i64> for PGType {
    fn from(i : i64) -> PGType {
        unsafe {
            PGType{ I64 : i }
        }
    }
}

impl Into<i64> for PGType {

    fn into(self) -> i64 {
        unsafe {
            self.I64
        }
    }
}

/*impl From<f64> for PGType {

}*/

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PGBytes {
    ptr : *const u8,
    len : usize
}

// cbindgen src/functions/pg_exec.rs -o /home/diego/Downloads/pg_exec.h --lang c

// Provides an API for the PostgreSQL executor function. The implementations match on what
// types were provided for PostgreSQL, and based on that dispatch to the correct dynamic symbol.
// Performs any type conversions that are necessary before calling the function.

pub type OneArgFn<'a, A, R> = Symbol<'a, unsafe extern fn(A)->Result<R,String>>;

pub type TwoArgFn<'a, A, B, R> = Symbol<'a, unsafe extern fn(A, B)->Result<R,String>>;

pub type ThreeArgFn<'a, A, B, C, R> = Symbol<'a, unsafe extern fn(A, B, C)->Result<R,String>>;

/*
/// The rust_exec is variadic in the number and types of arguments, so that the
/// types are correctly dispatched at runtime.
create function sum_two(a : real, b : real) as $$
    select rust_exec('my_func', cast(a as double precision), cast(b as double precision))
$$ language sql;
*/

/*#[repr(C)]
pub union PGSlice {
    Text : (*const u8, usize),
    Bytes : (*const u8, usize)
}*/

#[no_mangle]
pub extern "C" fn exec_pg(
    reg : *const PGRegistry, // Loaded when language is loaded; Preserved across all calls.
    func : usize,   // Loaded from database table
    values : *const PGType,
    n_args : usize,
    mut out : *mut PGType,
    mut out_msg : *mut c_char
) -> i32 {
    unsafe {
        let args = std::slice::from_raw_parts(values, n_args);
        match reg.as_ref().unwrap().call(func, args) {
            Ok(ans) => {
                *out = ans;
                0
            },
            Err(e) => {
                let s = CString::new(e).unwrap();
                out_msg = s.into_raw();
                1
            }
        }
    }
}

pub enum AnySymbol {
    OneArgI64(Box<dyn Fn(&dyn Any)->Result<i64, String>>),
    OneArgF64(Box<dyn Fn(&dyn Any)->Result<f64, String>>),
    OneArgText(Box<dyn Fn(&dyn Any)->Result<String, String>>),
    OneArgBytes(Box<dyn Fn(&dyn Any)->Result<Vec<u8>, String>>),
    TwoArgI64(Box<dyn Fn(&dyn Any, &dyn Any)->Result<i64, String>>),
    TwoArgF64(Box<dyn Fn(&dyn Any, &dyn Any)->Result<f64, String>>),
    TwoArgText(Box<dyn Fn(&dyn Any, &dyn Any)->Result<String, String>>),
    TwoArgBytes(Box<dyn Fn(&dyn Any, &dyn Any)->Result<Vec<u8>, String>>),
    ThreeArgI64(Box<dyn Fn(&dyn Any, &dyn Any, &dyn Any)->Result<i64,String>>),
    ThreeArgF64(Box<dyn Fn(&dyn Any, &dyn Any, &dyn Any)->Result<f64,String>>),
    ThreeArgText(Box<dyn Fn(&dyn Any, &dyn Any, &dyn Any)->Result<String,String>>),
    ThreeArgBytes(Box<dyn Fn(&dyn Any, &dyn Any, &dyn Any)->Result<Vec<u8>,String>>),
}

/// Hashmap agrees with function code at database table.
#[repr(C)]
pub struct PGRegistry {
    funcs : HashMap<usize, (Function, AnySymbol)>
}

impl PGRegistry {

    unsafe fn call(&self, func : usize, args : &[PGType]) -> Result<PGType, String> {
        let (f, sym) = &self.funcs[&func];
        let a1 = f.args[0];
        match sym {
            AnySymbol::OneArgI64(f) => match a1 {
                SqlType::I64 => f(&args[0].I64 as &dyn Any).map(|r| { let ty : PGType = r.into(); ty }),
                _ => unimplemented!()
            },
            _ => unimplemented!()
        }
    }

}

*/

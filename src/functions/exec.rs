use libloading::{Library, Symbol};
use crate::tables::{table::*, column::*};
use std::convert::{TryFrom, TryInto};
use super::loader::FunctionErr;
use super::sql_type::*;
use rusqlite::types::{ToSql, FromSql};
use rusqlite::functions::FunctionFlags;
use super::function::Function;
use std::any::Any;
use std::mem;

/*pub fn run(f : &Functioncols : Vec<Column>, types : &[SqlType]) {
    match cols.remove(0)
}*/

const EMPTY : Option<()> = None;

const EMPTY_VEC : Option<Vec<()>> = None;

/*impl PgRegistry {

    fn dispatch<T>(ptr : *const c_void, ty : SqlType) -> *const T {
        match ty {
            SqlType::I64 => mem::transmute::<_, *const i64>(ptr)
            SqlType::F64 => mem::transmute::<_, *const f64>(ptr)
            SqlType::String => mem::transmute::<_, *const u8>(ptr)
            SqlType::Bytes => mem::transmute::<_, *const u8>(ptr)
            => unimplemented!()
        }
    }

    pub unsafe fn exec(&self ix : i32, args : &[*const c_void]) -> Result<(), String> {
        match self.fns[ix] {
            OneArg(f) => {
                match self
            },
            _ => { }
        }
    }
}*/

/*fn load<'a, A, B, C, R>(
    lib : &'a Library,
    f : &'a Function
) -> Result<AnyFunction, String>
where
    A : ToSql + FromSql + Copy + 'static,
    B : ToSql + FromSql + Copy + 'static,
    C : ToSql + FromSql + Copy + 'static,
    R : ToSql + FromSql + 'static,
{
    let f_name = f.name.as_bytes();
    unsafe {
        match f.args.len() {
            1 => {
                let sym : Symbol<'a, unsafe extern fn(A)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                let raw_fn = sym.into_raw();
                let bx_f_any = Box::new(move |arg1 : &dyn Any| -> Result<Box<dyn Any>,String> {
                    let a : A = *arg1.downcast_ref().unwrap();
                    match raw_fn(a) {
                        Ok(res) => Ok(Box::new(res)),
                        Err(s) => Err(s)
                    }
                });
                Ok(AnyFunction::OneArg(bx_f_any))
            },
            _ => unimplemented!()
        }
    }
}*/

fn retrieve<'a, A, B, C, R>(
    conn : &rusqlite::Connection,
    lib : &'a Library,
    f : &'a Function
) -> Result<(), String>
    where
        A : ToSql + FromSql + 'static,
        B : ToSql + FromSql + 'static,
        C : ToSql + FromSql + 'static,
        R : ToSql + FromSql + 'static,
{
    let f_name = f.name.as_bytes();
    unsafe {
        match f.args.len() {
            1 => {
                let sym : Symbol<'a, unsafe extern fn(A)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                let raw_fn = unsafe { sym.into_raw() };
                conn.create_scalar_function(
                    &f.name,
                    f.args.len() as i32,
                    FunctionFlags::empty(),
                    move |ctx| {
                        let arg1 : A = ctx.get(0)?;
                        raw_fn(arg1).map_err(|e| rusqlite::Error::ModuleError(e))
                    }
                );
            },
            2 => {
                let sym : Symbol<'a, unsafe extern fn(A,B)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                let raw_fn = unsafe { sym.into_raw() };
                conn.create_scalar_function(
                    &f.name,
                    f.args.len() as i32,
                    FunctionFlags::empty(),
                    move |ctx| {
                        let arg1 : A = ctx.get(0)?;
                        let arg2 : B = ctx.get(1)?;
                        raw_fn(arg1, arg2).map_err(|e| rusqlite::Error::ModuleError(e))
                    }
                );
            },
            3 => {
                let sym : Symbol<'a, unsafe extern fn(A,B,C)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                let raw_fn = unsafe { sym.into_raw() };
                conn.create_scalar_function(
                    &f.name,
                    f.args.len() as i32,
                    FunctionFlags::empty(),
                    move |ctx| {
                        let arg1 : A = ctx.get(0)?;
                        let arg2 : B = ctx.get(1)?;
                        let arg3 : C = ctx.get(2)?;
                        raw_fn(arg1, arg2, arg3).map_err(|e| rusqlite::Error::ModuleError(e))
                    }
                );
            }
            _ => unimplemented!()
        }
    }
    Ok(())
}

#[inline(always)]
fn dispatch_ret<'a, A, B, C, R>(
    conn : &rusqlite::Connection,
    lib : &'a Library,
    f : &'a Function,
    n_arg : usize
) -> Result<(), String>
where
    A : ToSql + FromSql + 'static,
    B : ToSql + FromSql + 'static,
    C : ToSql + FromSql + 'static,
    R : ToSql + FromSql + 'static
{
    match f.ret {
        SqlType::I64 => retrieve::<A, B, C, i64>(conn, &lib, f),
        SqlType::F64 => retrieve::<A, B, C, f64>(conn, &lib, f),
        SqlType::Bytes => retrieve::<A, B, C, Vec<u8>>(conn, &lib, f),
        SqlType::String => retrieve::<A, B, C, String>(conn, &lib, f),
        _ => Err("Invalid SQL type".into())
    }
}

#[inline(always)]
fn dispatch_arg_at_pos<'a, A, B, C, R, T>(
    conn : &rusqlite::Connection,
    lib : &'a Library,
    arg_ix : usize,
    f : &'a Function
) -> Result<(), String>
where
    A : ToSql + FromSql + 'static,
    B : ToSql + FromSql + 'static,
    C : ToSql + FromSql + 'static,
    R : ToSql + FromSql + 'static,
    T : ToSql + FromSql + 'static
{
    // We use i64 at the 0 and 1 variants as a placeholder, only so that it satisfies ToSql.
    // The actual value is not used if n_args is less than the required number. The compiler
    // will specialize those placeholders, but the program will never branch into them at runtime.
    match arg_ix {
        0 => dispatch_arg::<T, i64, i64, R>(conn,&lib, arg_ix+1, &f),
        1 => dispatch_arg::<A, T, i64, R>(conn,&lib, arg_ix+1, &f),
        2 => dispatch_arg::<A, B, T, R>(conn, &lib, arg_ix+1, &f),
        _ => Err(String::from("Invalid argument position"))
    }
}

#[inline(always)]
fn dispatch_arg<'a, A, B, C, R>(
    conn : &rusqlite::Connection,
    lib : &'a Library,
    arg_ix : usize,
    f : &'a Function
) -> Result<(), String>
where
    A : ToSql + FromSql + 'static,
    B : ToSql + FromSql + 'static,
    C : ToSql + FromSql + 'static,
    R : ToSql + FromSql + 'static
{
    match f.args.get(arg_ix) {
        Some(SqlType::I64) => dispatch_arg_at_pos::<A,B,C,R,i64>(conn, lib, arg_ix, f),
        Some(SqlType::Bytes) => dispatch_arg_at_pos::<A,B,C,R,Vec<u8>>(conn,lib, arg_ix, f),
        Some(SqlType::F64) => dispatch_arg_at_pos::<A,B,C,R,f64>(conn,lib, arg_ix, f),
        Some(SqlType::String) => dispatch_arg_at_pos::<A,B,C,R,String>(conn,lib, arg_ix, f),
        None => dispatch_ret::<A, B, C, R>(conn, lib, f, arg_ix),
        _ => panic!("Invalid type")
    }
}

pub fn register_from_dylib<'a, A, B, C, R>(
    conn : &rusqlite::Connection,
    lib : &'a Library,
    arg_ix : usize,
    f : &'a Function
) -> Result<(), String>
where
        A : ToSql + FromSql + 'static,
        B : ToSql + FromSql + 'static,
        C : ToSql + FromSql + 'static,
        R : ToSql + FromSql + 'static
{
    // We use i64 as a placeholder, only so that it satisfies ToSql.
    // The dispatch_arg is called recursively to fill in for the values
    // of f.
    dispatch_arg::<i64, i64, i64, i64>(conn, lib, 0, f)
}



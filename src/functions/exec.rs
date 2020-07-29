use libloading::{Library, Symbol};
use super::parser::*;
use crate::tables::{table::*, column::*};
use ::queries::*;
use std::convert::{TryFrom, TryInto};
use super::loader::FunctionErr;

/*pub fn run(f : &Functioncols : Vec<Column>, types : &[SqlType]) {
    match cols.remove(0)
}*/

/*const EMPTY : Option<()> = None;

const EMPTY_VEC : Option<Vec<()>> = None;

fn call<'a, A,B,C,/*D,E,*/R>(
    lib : &'a Library,
    f_name : &[u8],
    a : A,
    b : Option<B>,
    c : Option<C>,
    // d : Option<D>,
    // e : Option<E>
) -> Result<R, String>
    where
        A : 'a,
        B : 'a,
        C : 'a,
        //D : 'a,
        //E : 'a,
        R : 'a
{
    unsafe {
        match (b, c /*, d, e*/ ) {
            (None, None, /*None, None*/ ) => {
                let f : Symbol<'a, unsafe extern fn(A)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                f(a)
            },
            (Some(b), None, /*None, None*/ ) => {
                let f : Symbol<'a, unsafe extern fn(A, B)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                f(a,b)
            },
            (Some(b), Some(c), /*None, None*/ ) => {
                let f : Symbol<'a, unsafe extern fn(A,B,C)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                f(a, b, c)
            },
            /*(Some(b), Some(c), Some(d), None) => {
                let f : Symbol<'a, unsafe extern fn(A,B,C,D)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                f(a, b, c, d)
            },
            (Some(b), Some(c), Some(d), Some(e)) => {
                let f : Symbol<'a, unsafe extern fn(A,B,C,D,E)->Result<R,String>> =
                    lib.get(f_name).map_err(|e| format!("{}",e))?;
                f(a, b, c, d, e)
            },*/
            _ => unimplemented!()
        }
    }
}

#[inline(always)]
fn dispatch_ret_at_pos<A, B, C, /*D, E, */ R1, R2, R3, /*R4, R5, */ T>(
    lib : &Library,
    f : &Function,
    a : A,
    b : Option<B>,
    c : Option<C>,
    // d : Option<D>,
    // e : Option<E>,
    ret_ix : usize
) -> Result<Vec<Column>, String>
where
    Column : From<Vec<R1>> + From<Vec<R2>> + From<Vec<R3>> /*+ From<Vec<R4>> + From<Vec<R5>>*/ + From<Vec<T>>
{
    match ret_ix {
        0 => dispatch_ret::<A,B,C, /*,D,E,*/ T,(),(),/*(),()*/>(lib, f,  a, b, c, /*d, e,*/ ret_ix+1),
        1 => dispatch_ret::<A,B,C,/*D,E,*/R1, T,(),/*(),()*/>(lib, f,  a, b, c, /*d, e,*/ ret_ix+1),
        2 => dispatch_ret::<A,B,C,/*D,E,*/R1, R2, T, /*(), ()*/>(lib, f,  a, b, c, /*d, e,*/ ret_ix+1),
        // 3 => dispatch_ret::<A,B,C,D,E,R1, R2, R3, T, ()>(lib, f,  a, b, c, d, e, ret_ix+1),
        // 4 => dispatch_ret::<A,B,C,D,E,R1, R2, R3, R4, T>(lib, f, a, b, c, d, e, ret_ix+1),
        _ => unimplemented!()
    }
}

fn dispatch_ret<A, B, C, /*D, E,*/ R1, R2, R3, /*R4, R5*/>(
    lib : &Library,
    f : &Function,
    a : A,
    b : Option<B>,
    c : Option<C>,
    //d : Option<D>,
    //e : Option<E>,
    ret_ix : usize
) -> Result<Vec<Column>, String>
where
    Column : From<Vec<R1>> + From<Vec<R2>> + From<Vec<R3>> //+ From<Vec<R4>> + From<Vec<R5>>
{
    match f.ret.get(ret_ix) {
        Some(SqlType::I32) => dispatch_ret_at_pos::<_, _, _, /*_, _,*/ R1, R2, R3, /*R4, R5,*/ i32>(&lib, &f, a, b, c, /*d, e,*/ ret_ix),
        Some(SqlType::I64) => dispatch_ret_at_pos::<_, _, _, /*_, _,*/ R1, R2, R3, /*R4, R5,*/ i64>(&lib, &f, a, b, c, /*d, e,*/ ret_ix),
        Some(SqlType::F32) => dispatch_ret_at_pos::<_, _, _, /*_, _,*/ R1, R2, R3, /*R4, R5,*/ f32>(&lib, &f, a, b, c, /*d, e,*/ ret_ix),
        Some(SqlType::F64) => dispatch_ret_at_pos::<_, _, _, /*_, _,*/ R1, R2, R3, /*R4, R5,*/ f64>(&lib, &f, a, b, c, /*d, e,*/ ret_ix),
        Some(SqlType::String) => dispatch_ret_at_pos::<_, _, _, /*_, _,*/ R1, R2, R3, /*R4, R5,*/ String>(&lib, &f, a, b, c, /*d, e,*/ ret_ix),
        None => {
            let f_name = f.name.as_bytes();
            let var_ret = f.var_ret;
            match ret_ix {
                1 => {
                    match var_ret {
                        true => {
                            let ans = call::<A, B, C, /*D, E,*/ Vec<Vec<R1>>>(lib, f_name, a, b, c, /*d, e*/)?;
                            Ok(ans.into_iter().map(|v| v.into()).collect())
                        }
                        false => {
                            let ans = call::<A, B, C, /*D, E,*/ Vec<R1>>(lib, f_name, a, b, c, /*d, e*/)?;
                            Ok(vec![ans.into()])
                        }
                    }
                },
                2 => {
                    match var_ret {
                        true => {
                            let (fix, var) =
                                call::<A, B, C, /*D, E,*/ (Vec<R1>,Vec<Vec<R2>>)>(lib, f_name, a, b, c, /*d, e*/)?;
                            let mut cols = vec![fix.into()];
                            cols.extend(var.into_iter().map(|v| v.into()));
                            Ok(cols)
                        }
                        false => {
                            let fix = call::<A, B, C, /*D, E,*/ (Vec<R1>,Vec<R2>)>(lib, f_name, a, b, c, /*d, e*/)?;
                            Ok(vec![fix.0.into(), fix.1.into()])
                        }
                    }
                },
                3 => {
                    match var_ret {
                        true => {
                            let (fix1, fix2, var) =
                                call::<A, B, C, /*D, E,*/ (Vec<R1>,Vec<R2>,Vec<Vec<R3>>)>(lib, f_name, a, b, c, /*d, e*/)?;
                            let mut cols = vec![fix1.into(), fix2.into()];
                            cols.extend(var.into_iter().map(|v| v.into()));
                            Ok(cols)
                        }
                        false => {
                            let fix = call::<A, B, C, /*D, E,*/ (Vec<R1>,Vec<R2>,Vec<R3>)>(lib, f_name, a, b, c, /*d, e*/)?;
                            Ok(vec![fix.0.into(), fix.1.into(), fix.2.into()])
                        }
                    }
                },
                /*4 => {
                    match var_ret {
                        true => {
                            let (fix1, fix2, fix3, var) =
                                call::<A, B, C, D, E, (Vec<R1>,Vec<R2>,Vec<R3>,Vec<Vec<R4>>)>(lib, f_name, a, b, c, d, e)?;
                            let mut cols = vec![fix1.into(), fix2.into(), fix3.into()];
                            cols.extend(var.into_iter().map(|v| v.into()));
                            Ok(cols)
                        }
                        false => {
                            let fix = call::<A, B, C, D, E, (Vec<R1>,Vec<R2>,Vec<R3>,Vec<R4>)>(lib, f_name, a, b, c, d, e)?;
                            Ok(vec![fix.0.into(), fix.1.into(), fix.2.into(), fix.3.into()])
                        }
                    }
                },
                5 => {
                    match var_ret {
                        true => {
                            let (fix1, fix2, fix3, fix4, var) =
                                call::<A, B, C, D, E, (Vec<R1>,Vec<R2>,Vec<R3>,Vec<R4>,Vec<Vec<R5>>)>(lib, f_name, a, b, c, d, e)?;
                            let mut cols = vec![fix1.into(), fix2.into(), fix3.into(), fix4.into()];
                            cols.extend(var.into_iter().map(|v| v.into()));
                            Ok(cols)
                        }
                        false => {
                            let fix = call::<A, B, C, D, E, (Vec<R1>,Vec<R2>,Vec<R3>,Vec<R4>, Vec<R5>)>(lib, f_name, a, b, c, d, e)?;
                            Ok(vec![fix.0.into(), fix.1.into(), fix.2.into(), fix.3.into(), fix.4.into()])
                        }
                    }
                },*/
                _ => unimplemented!()
            }
        }
    }
}

fn take_variadic_arg<V>(
    last_fix : V,
    fix_ix : usize,
    mut rem_args : Vec<Column>
) -> Result<Vec<V>, FunctionErr>
    where
        // F : Into<Vec<V>>,
        Column : TryInto<V>
{
    let mut var_args = vec![last_fix];
    let mut var_ix = fix_ix;
    while rem_args.len() > 0 {
        let res_v : Result<V,_> = rem_args.remove(0).try_into();
        match res_v {
            Ok(v) => var_args.push(v),
            Err(_) => { return Err(FunctionErr::TypeMismatch(var_ix)); }
        }
        var_ix += 1;
    }
    Ok(var_args)
}

#[inline(always)]
fn dispatch_arg_at_pos<A, B, C, /*D, E,*/ T>(
    lib : &Library,
    f_name : &[u8],
    a : A,
    b : B,
    c : C,
    // d : D,
    // e : E,
    arg_ix : usize,
    mut cols : Vec<Column>,
    f : &Function
) -> Result<Vec<Column>, FunctionErr>
    where
        Column : TryInto<A> + TryInto<B> + TryInto<C> /*+ TryInto<D> + TryInto<E>*/ + TryInto<T>
{
    let res_v : Result<T,_> = cols.remove(0).try_into();
    match res_v {
        Ok(v) => match arg_ix {
            0 => dispatch_arg(&lib, f.name.as_bytes(), v, (), (), /*(), (),*/ arg_ix+1, cols, &f),
            1 => dispatch_arg(&lib, f.name.as_bytes(), a, v, (), /*(), (),*/ arg_ix+1, cols, &f),
            2 => dispatch_arg(&lib, f.name.as_bytes(), a, b, v, /*(), (),*/ arg_ix+1, cols, &f),
            //3 => dispatch_arg(&lib, f.name.as_bytes(), a, b, c, v, (), arg_ix+1, cols, &f),
            //4 => dispatch_arg(&lib, f.name.as_bytes(), a, b, c, d, v, arg_ix+1, cols, &f),
            _ => unimplemented!()
        },
        Err(_) => return Err(FunctionErr::TypeMismatch(arg_ix))
    }
}

pub fn dispatch_arg<A, B, C, /*D, E*/>(
    lib : &Library,
    f_name : &[u8],
    a : A,
    b : B,
    c : C,
    //d : D,
    //e : E,
    arg_ix : usize,
    mut cols : Vec<Column>,
    f : &Function
) -> Result<Vec<Column>, FunctionErr>
    where
        Column : TryInto<A> + TryInto<B> + TryInto<C> //+ TryInto<D> + TryInto<E>
{
    match f.args.get(arg_ix) {
        Some(SqlType::I32) => dispatch_arg_at_pos::<_,_,_,/*_,_,*/ Vec<i32>>(lib, f.name.as_bytes(), a, b, c, /*d, e,*/ arg_ix, cols, f),
        Some(SqlType::I64) => dispatch_arg_at_pos::<_,_,_,/*_,_,*/ Vec<i64>>(lib, f.name.as_bytes(), a, b, c, /*d, e,*/ arg_ix, cols, f),
        Some(SqlType::F32) => dispatch_arg_at_pos::<_,_,_,/*_,_,*/ Vec<f32>>(lib, f.name.as_bytes(), a, b, c, /*d, e,*/ arg_ix, cols, f),
        Some(SqlType::F64) => dispatch_arg_at_pos::<_,_,_,/*_,_,*/ Vec<f64>>(lib, f.name.as_bytes(), a, b, c, /*d, e,*/ arg_ix, cols, f),
        Some(SqlType::String) => dispatch_arg_at_pos::<_,_,_,/*_,_,*/ Vec<String>>(lib, f.name.as_bytes(), a, b, c, /*d, e,*/ arg_ix, cols, f),
        None => {
            match arg_ix {
                1 => match f.var_arg {
                    true => {
                        let mut var_a = take_variadic_arg(a, 1, cols)?;
                        dispatch_ret::<_,_,_,/*_,_,*/(),(),(),/*(),()*/>(&lib, &f, var_a, EMPTY, EMPTY, /*EMPTY, EMPTY,*/ 0)
                            .map_err(|msg| FunctionErr::UserErr(msg))
                    },
                    false => dispatch_ret::<_,_,_,/*_,_,*/(),(),(),/*(),()*/>(&lib, &f, a, EMPTY, EMPTY, /*EMPTY, EMPTY,*/ 0)
                        .map_err(|msg| FunctionErr::UserErr(msg)),
                },
                2 => unimplemented!(),
                3 => unimplemented!(),
                4 => unimplemented!(),
                5 => unimplemented!(),
                _ => unimplemented!()
            }
        }
    }
}*/




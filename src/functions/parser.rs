use syn::{File, Item, Type, ItemFn, ItemMod, ReturnType, FnArg, AttrStyle, Visibility };
use syn::parse::Parse;
use quote::{ToTokens};
use proc_macro2::TokenStream;
use std::env;
use toml::Value;
use chrono::{DateTime, offset::Utc};
use std::io::Read;
use std::fs;
use std::path::Path;
use std::fmt::{self, Debug, Display};
use std::any::{Any, TypeId};
use std::default::Default;
use std::convert::{TryFrom, TryInto};
use super::sql_type::*;
use crate::tables::column::*;
use libloading::{Library, Symbol};
use super::loader::*;
use crate::tables::column::*;
use super::function::*;
use std::ffi::OsStr;
use std::path::PathBuf;
use crate::utils;

#[derive(Debug, Clone)]
pub enum FunctionMode {
    Simple,
    Aggregate,
    Window,
    Invalid
}

impl TryFrom<String> for FunctionMode {

    type Error = ();

    fn try_from(s : String) -> Result<FunctionMode, ()> {
        match &s[..] {
            "Simple" => Ok(FunctionMode::Simple),
            "Aggregate" => Ok(FunctionMode::Aggregate),
            "Window" => Ok(FunctionMode::Window),
            "Invalid" => Ok(FunctionMode::Invalid),
            _ => Err(())
        }
    }

}

impl fmt::Display for FunctionMode {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            FunctionMode::Simple => "Simple",
            FunctionMode::Aggregate => "Aggregate",
            FunctionMode::Window => "Window",
            FunctionMode::Invalid => "Invalid"
        };
        write!(f, "{}", name)
    }

}

// Using now function::Function
/*#[derive(Debug, Clone)]
pub struct Function {
    pub name : String,
    pub args : Vec<SqlType>,
    pub ret : Vec<SqlType>,
    pub doc : Option<String>,
    pub mode : FunctionMode,
    pub var_arg : bool,
    pub var_ret : bool
}

impl Function {

    pub fn call<'a>(&'a self, lib : &'a Library, cols : Vec<Column>) -> Result<Vec<Column>, FunctionErr> {
        unsafe {
            let f : Symbol<'a, unsafe extern fn(Vec<Column>)->Result<Vec<Column>,String>> =
                lib.get(self.name.as_bytes()).map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
            let ans = f(cols).map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
            Ok(ans)
        }
    }

    /// Hints which columns could not have its name retrieved on error.
    pub fn get_col_names(&self, arg_names : Vec<String>) -> Result<Vec<String>, FunctionErr> {
        // Match column name to pattern informed by user.
        unimplemented!()
    }


}*/

/*/// Gets type and return whether the type was nested into Vec<Vec<T>>
fn get_simple_type(ts : TokenStream) -> Option<(SqlType, bool)> {
    let ty_str : String = format!("{}", ts)
        .chars().filter(|c| !c.is_whitespace()).collect();
    let agg_ty : SqlAggType = (&ty_str[..]).try_into().ok()?;
    let nested = match agg_ty {
        SqlAggType::NestedVec(_) => true,
        SqlAggType::Vec(_) => false
    };
    Some((agg_ty.inner(), nested))
}*/

pub fn load_doc(item_fn : &ItemFn) -> Option<String> {
    let mut doc_content = String::new();
    for attr in &item_fn.attrs {
        let ident = attr.path.get_ident().to_token_stream().to_string();
        match &ident[..] {
            "doc" => doc_content += &attr.tokens.to_string()[..],
            _ => { }
        }
    }
    doc_content = doc_content.clone().chars()
        .filter(|c| *c != '"' && *c != '=')
        .collect();
    doc_content = doc_content.clone().trim_matches(' ').to_string();
    utils::break_string(&mut doc_content, 45);
    if doc_content.len() == 0 {
        None
    } else {
        Some(doc_content)
    }
}

/// Returns (name, arg types, return type)
pub fn function_signature(f : ItemFn) -> Result<Function, String> {
    let name = f.sig.ident.to_token_stream().to_string();
    let inputs = f.sig.inputs.iter();
    let mut args : Vec<SqlType> = Vec::new();
    // let mut mode = FunctionMode::Simple;
    let mut doc_content = String::new();
    let var_arg = false;
    let var_ret = false;
    for input in inputs {
        match input {
            FnArg::Typed(typed) => {
                //let (ty, nested) = get_simple_type(typed.ty.to_token_stream())?;
                let ty_str = format!("{}", typed.ty.to_token_stream());
                let ty : SqlType = (&ty_str[..]).try_into()
                    .map_err(|_| format!("Invalid argument type for function {}: {}", name, ty_str))?;
                args.push(ty);
                // var_arg = nested;
            },
            _ => {  }
        }
    }
    /*for attr in f.attrs {
                let ident = attr.path.get_ident().to_token_stream().to_string();
                match &ident[..] {
                    "doc" => doc_content += &attr.tokens.to_string()[..],
                    _ => { }
                }
                //let tokens = attr.tokens.to_string();
    }*/
    let doc = load_doc(&f);
    // let mut ret : Option<SqlType> = None;
    match f.sig.output {
        ReturnType::Type(_, bx_type) => {
            /*match *bx_type {
                syn::Type::Tuple(tuple) => {
                    for t in tuple.elems.iter() {
                        let (ty, nested) = get_simple_type(t.to_token_stream())?;
                        ret.push(ty);
                        var_ret = nested;
                    }
                },
                ty => {
                    let (ty, nested) = get_simple_type(ty.to_token_stream())?;
                    ret.push(ty);
                    var_ret = nested;
                }
            }*/
            let ty_str = format!("{}", bx_type.to_token_stream());

            let result_err = "Return type must be Result<T,String> for T a String, f64, i64 or Vec<u8>";
            let is_result = ty_str.split("<").next().map(|s| s.trim()) == Some("Result");
            let is_err = ty_str.split(",").nth(1).map(|s| s.trim()) == Some("String >");
            println!("is_result = {}; is_err = {}", is_result, is_err);
            if !is_result || !is_err {
                return Err(String::from(result_err));
            }
            let inner_ty = ty_str
                .split(|c| if c == '<' || c == '>' || c == ',' { true } else { false })
                .nth(1)
                .unwrap()
                .to_string();
            let ret : SqlType = (&inner_ty[..]).try_into()
                .map_err(|_| format!("Invalid return type for function {}: {}", name, ty_str))?;
            Ok( Function{ name, args, ret, doc, /*mode,*/ var_arg /*, var_ret*/ } )
        },
        _ => Err(format!("Invalid return type"))
    }
}

/// Apply function_signature to a module and all its submodules recursively.
pub fn parse_mod_signature(
    item_mod : ItemMod
) -> Result<Vec<Function>, String> {
    let mut sigs = Vec::new();
    if let Some((_, items)) = item_mod.content {
        for item in items {
            match item {
                Item::Mod(item_mod) => {
                    match item_mod.vis {
                        Visibility::Public(_) => { sigs.extend(parse_mod_signature(item_mod)?); }
                        _ => { }
                    }
                },
                Item::Fn(item_fn) => {
                    match item_fn.vis {
                        Visibility::Public(_) => { sigs.push(function_signature(item_fn)?); },
                        _ => { }
                    }
                },
                _ => { }
            }
        }
    }
    Ok(sigs)
}

/// Takes any item and retrieves the signature of all functions,
/// running over modules recursively.
pub fn parse_fn_or_mod(
    item : Item
) -> Result<Vec<Function>, String> {
    let mut sigs = Vec::new();
    match item {
        Item::Mod(item_mod) => {
            match item_mod.vis {
                Visibility::Public(_) => sigs.extend(parse_mod_signature(item_mod)?),
                _ => { }
            }
        },
        Item::Fn(item_fn) => {
            match item_fn.vis {
                Visibility::Public(_) => sigs.push(function_signature(item_fn)?),
                _ => { }
            }
        },
        _ => { }
    }
    Ok(sigs)
}

pub fn parse_top_level_funcs(
    content : &str
) -> Result<Vec<Function>, String> {
    let t : File = syn::parse_str(content).map_err(|e| format!("{}", e) )?;
    let mut sigs = Vec::new();
    for item in t.items {
        match item {
            Item::Fn(item_fn) => {
                match item_fn.vis {
                    Visibility::Public(_) => sigs.push(function_signature(item_fn)?),
                    _ => { },
                }
            },
            _ => { }
        }
    }
    Ok(sigs)
}

/// Parse a full source file applying parse_fn_or_mod.
/// Return function name; arguments; return.
pub fn parse_nested_signatures(
    content : &str
) -> Result<Vec<Function>, String> {
    let t : File = syn::parse_str(content).map_err(|e| format!("{}", e) )?;
    let mut sigs = Vec::new();
    for item in t.items {
        sigs.extend(parse_fn_or_mod(item)?);
    }
    Ok(sigs)
}

pub fn search_sources_in_crate(p : &Path) -> Vec<PathBuf> {
    if p.is_dir() {
        let mut sources = Vec::new();
        let paths = fs::read_dir(p).unwrap();
        for p in paths.filter_map(|p| p.ok() ) {
            if p.path().is_dir() {
                sources.extend(search_sources_in_crate(&p.path()));
            }
            if p.path().extension() == Some(OsStr::new("rs")) {
                sources.push(p.path());
            }
        }
        sources
    } else {
        println!("Tried to search for sources in non-dir path");
        Vec::new()
    }
}

#[test]
fn parse_test() -> Result<(),()> {
    let test = r#"
        pub mod internal {

            /// This is an internal function
            /// This is another doc line for internal function.
            #[sql_agg]
            pub fn sum_in(d : f32)->f32 { 0.0 }
        }

        /// This is an external function
        #[sql]
        pub fn sum_out(txt : String, args : f32)->String { txt } "#;
    let funcs = parse_nested_signatures(test).unwrap();
    for f in funcs.iter() {
        println!("{:?}", f);
    }
    Ok(())
}

/*pub struct Crate {
    name : String,
    compile_date : Option<DateTime<Utc>>,
    funcs : Vec<Function>,
    path : String,
    lib : Library
}

impl Crate {

    fn function_list(&self) -> Vec<&'a Function> {
        self.funcs.iter().collect()
    }

    fn load(path : &str) -> Result<Self, String> {
        unimplemented!()
    }

    fn call_single_arg<'a, A, R>(
        &'a self,
        name : &str
    ) -> Result<OneArgSymb<'a, A, R>, String> {
        let f : OneArgSymb<'a, A, B> = lib.get(name).map(|e| format!("{}", e) )?;
    }

    fn call_one_arg<'a, A, R>(
        OneArgSymb<'a, A, R>,
        tbl : Table
    ) -> Result<Table, String> {
        let mut ans : Vec<A> = Vec::new();
        for e in table.column(0).iter() {
            ans.push(f(e));
        }
        let tbl_ans = Table::new();
        tbl_ans.add_column(ans);
        Ok(tbl_ans);
    }

    fn dispatch_single<A,R>(arg : A, ret : R) -> OneArgSymb<'a,A,R>
    where
        A : Display,
        B : Display
    {
        match (&format!("{}", arg)[..], &format!("{}", ret)[..]) {
            ("Integer", "Integer") => { call_one_arg::<String, f64>(lib, self.name, tbl) },
            ("Integer", "Real") => { call_one_arg::<String, i32>(lib, self.name, tbl) },
            ("Integer", "Text") => { call_one_arg::<String, f64>(lib, self.name, tbl) },
            ("Integer", "Bytes") => { call_one_arg::<String, Vec<u8>>(lib, self.name, tbl) }
        }
    }

    /// Caller match on B
    fn dispatch_tuple_ret<A,B,R>(ret : R)
    where
        R : Display
    {
        match &format!("{}", ret)[..] {
            "Integer" => Self::call_two_arg::<A, B, i32>(),
            "Real" => Self::call_two_arg::<A, B, f64>(),
            "Text" => Self::call_two_arg::<A, B, String>(),
            "Bytes" => Self::call_two_arg::<A, B, Vec<u8>>(),
            _ => unimplemented!()
        }
    }

    /// Caller match on B
    fn dispatch_tuple_arg2<A,B,R>(arg2 : B, ret : R)
    where
        B : Display, R : Display
    {
        match &format!("{}", arg2)[..] {
            "Integer" => Self::dispatch_tuple_ret::<A, i32, R>(ret),
            "Real" => Self::dispatch_tuple_ret::<A, f64, R>(ret),
            "Text" => Self::dispatch_tuple_ret::<A, String, R>(ret),
            "Bytes" => Self::dispatch_tuple_ret::<A, Vec<u8>, R>(ret),
            _ => unimplemented!()
        }
    }

    fn dispatch_tuple_arg1<A, B, R>(arg1 : A, arg2 : B, ret : R)
    where
        A : Display, B : Display, R : Display
    {
        match &format!("{}", arg1)[..] {
            "Integer" => Self::dispatch_tuple_arg2::<i32, B, R>(arg2, ret),
            "Real" => Self::dispatch_tuple_arg2::<f64, B, R>(arg2, ret),
            "Text" => Self::dispatch_tuple_arg2::<String, B, R>(arg2, ret),
            "Bytes" => Self::dispatch_tuple_arg2::<Vec<u8>, B, R>(arg2, ret),
            _ => unimplemented!()
        }
    }
    /// Caller match on C
    fn dispatch_triple<A, B, C, R>(arg1 : A, arg2 : B, arg3 : C) -> ThreeArgymb<A, B, C, R> {
        match &format!("{}", arg3)[..] {
            "Integer" => call_three_arg::<A, B, i32, R>()
            "Real" => call_three_arg::<A, B, f64, R>()
            "Text" => call_three_arg::<A, B, String, R>
            "Bytes" => call_three_arg::<A, B, Vec<u8>, R>
        }
    }

    fn dispatch_single_arg(lib : &Library) -> Result<Table, String> {
        match (self.args[0], self.args[1]) {
            ("String", "f64") => { call_one_arg::<String, f64>(lib, self.name, tbl) },
            ("String", "i32") => { call_one_arg::<String, i32>(lib, self.name, tbl) },
            ("String", "f64") => { call_one_arg::<String, f64>(lib, self.name, tbl) },
            ("String", "Vec<u8>") => { call_one_arg::<String, Vec<u8>>(lib, self.name, tbl) }
        }
    }

    fn call(func : &str, tbl : Table) -> Result<Table, String> {
        let func = self.funcs.find(|f| f.name == func).unwrap();
        match (func.mode, func.args.len()) {
            (FunctionMode::Simple, 1) => {

            },
            (FunctionMode::Aggregate, 1) => {

            },
            (FunctionMode::Simple, 2) => {

            },
            (FunctionMode::Aggregate, 2) => {

            },
            (FunctionMode::Simple, 3) => {

            },
            (FunctionMode::Aggregate, 3) => {

            }
            _ => {
                unimplemented!()
            }
        }
    }

}*/

/*/// Returns the crate name and the last date it was
/// compiled (if it was compiled to .so at any point)
pub fn info_from_toml(path : Path) -> Crate {
    let mut content = String::new();
    let f = File::open(path).unwrap();
    f.read_to_string(&mut content);
    let parent = path.parent();
    let info : Value = f.parse().map_err(|e| {
        println!("{}", e);
        return (String::from(""), None)
    });
}

pub fn search_rust_crates(dir : &Path) -> Vec<Crate> {
    let mut crates = Vec::new();
    if dir.is_dir() {
        let paths = fs::read_dir(dir).unwrap();
        for p in paths {
            if p.path().is_dir() {
                crates.extend(search_rust_crates(p.path()));
            } else {
                if p.path().extension() == Some(OsStr::new("toml")) {
                    sources.push(info_from_toml(p));
                }
            }
        }
    }
    crates
}

pub fn extract_sources() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let var = env::vars().iter().find(|(k, v)| k == "QUERIES_PATH")
        .ok_or(String::from("QUERIES_PATH not set"))?;
    let dirs : Vec<_> = var.split(':').collect();
    for dir in dirs {

    }

        for path in paths {
            if let Ok(p) = path {

                    names.push(p.path().to_str()
                        .ok_or("Error converting path to str")?
                        .to_string()
                    );
                } else {
                    return Err("Non-png path found at directory");
                }
            } else {
                return Err("Unable to recover path");
            }
        }
}*/


/*pub struct Call<A,B,C,D,E,R>(A,B,C,D,E,R);
// &["i32,"i64","i64","i32"]
impl<A,B,C,R> TryFrom<&[str]> for Call<i32, i64, i64,i32> {
}
// varargs are admissible as long as all types have the same type, so they can be
// packed into a slice. The corresponding aggregate must implement &[&[f64]]->f64/JSON,
// which is an iterator over the aggregated **columns**. The window will follow a similar procedure,
// but have a (&[&[f64]], f64) signature.
// Should provide
// #[sql] Ordinary functions
// #[sql_agg] Aggregates
// #[sql_win] Windows
// #[sql_vtab] Virtual tables.
impl<A,B,C,R> TryFrom<&[str]> for Call<&[f64], (), (), (), f64> {

}

// &["i32"]
impl<A,B,C,R> TryFrom<&[str]> for Call<i32, (), (), ()> {

}

pub trait SqlCall<A,B,C,D,E,F>
    where
        A : Any
{

    fn sql_agg_call(&'a self, lib : &'a Library, name : &str, tbl : Table) -> Result<Table, String> {
        let f : OneArgSymb<'a, Vec<A>, Vec<B>> = lib.get(name)?;
        let ans = f(table.column(0), table.column(1))
        Ok(Table::from(ans))
    }

    fn sql_simple_call(&'a self, lib : &'a Library, name : &str, tbl : Table) -> Result<Table, String> {
        let f : OneArgSymb<'a, A, B> = lib.get(name)?;
        let mut ans : Vec<A> = Vec::new();
        for e in table.column(0).iter() {
            ans.push(f(e));
        }
        let tbl_ans = Table::new();
        tbl_ans.add_column(ans);
        Ok(tbl_ans);
    }

}

impl SqlCall<f32,(),(),(),(),f32> for Call<f32, f32> { }

impl SqlCall<f32,(),(),(),(),Result<f32, String>> for Call<f32, Result<f32, String> { }

pub struct Call<A, B, C, D, E, R> {
    name : String,
    arg1 : A
    arg2 : B
    arg3 : C
    arg4 : D
    arg5 : E
    ret : R
}

impl<A,R> TryFrom<(String, Vec<String>, String)> for Call<A, (), (), (), (), R> {

}

fn type_match<T>(t : &str) -> bool
where T : Any
{
    match t {
        "String" => TypeId::of::<T>() == TypeId::of::<String>(),
        "f32" => TypeId::of::<T>() == TypeId::of::<f32>(),
        "f64" => TypeId::of::<T>() == TypeId::of::<f64>(),
        "i32" => TypeId::of::<T>() == TypeId::of::<i32>(),
        "Vec<u8>" => TypeId::of::<T>() == TypeId::of::<Vec<u8>>(),
        _ => false
    }
}

//fn parse_two<'a, A, B, R>(lib : &'a Library, name : &str, args : [&str; 2], ret : &str) -> TwoArgSymb<'a, A, B, R> {
//}

trait AnyFunc<F>
    where Self : FromStr
{

    fn build(lib : Library, name : &str, types : [&str]) -> F;
}

impl AnyFunc<OneArgSymb> for Call<A,(),(),(),R> {

    fn (lib : Library, name : &str, types : [&str]) -> OneArgSymb<A,R>;

    }
}

impl AnyFunc<TwoArgSymb> for Call<A,B,(),(),R> {

}

impl AnyFunc<ThreeArgSymb> for Call<A,B,C,(),R> {

}

impl AnyFunc<FourArgSymb> for Call<A,B,C,D,R> {

}

// After the first call, save the function to the registry with the respective types.
// Give user update button to re-load dynamic libraries.

impl<A,B,C,D,E,R> Call<A,B,C,D,E,R>
    Self : TryFrom<&[str]>
{

    fn call(types : &[str]) -> Result<R, &'static str> {
        let types = &["i32", "f64", "String", "Vec<u8>"];
        let comb_2 = &[0, 1];
        let comb_3 = &[0, 1, 2];
        let comb_4 = &[0, 1, 2, 3];

        let n_args = &[1,2,3,4,5];
        for n in n_args.iter() {
            for t1 in types {
                for p2 in &types[n..n_args] {
                    for p3 &types[n..n_args] {
                        for p4 in &types[n..n_args] {
                            match SqliteCall::try_from(&types[0..n_args]) {
                                Ok(call) => return call.build().call(),
                                Err(_) => _
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_two(types : &str) -> TwoArgSymb<A,B,R> {

    }

}
}*/


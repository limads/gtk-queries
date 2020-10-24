use syn::{self, File, Item, Type, ItemFn, ItemMod, ReturnType, FnArg, AttrStyle, Visibility };
use syn::parse::Parse;
use quote::{ToTokens};
use proc_macro2::TokenStream;
use std::env;
use toml::Value;
use std::io::Read;
use std::fs;
use std::path::Path;
use std::fmt::{self, Debug, Display};
use std::any::{Any, TypeId};
use std::default::Default;
use std::convert::{TryFrom, TryInto};
use super::sql_type::*;
use std::error::Error;
use std::cmp::PartialEq;
use std::str::FromStr;
use rusqlite::{self, functions::Context };
use libloading::Symbol;
use std::collections::HashMap;
use super::parser;

/*#[derive(Debug, Clone, PartialEq)]
pub enum FunctionMode {

    Simple,

    /// Holds (Init, State, Final) triple
    Aggregate(String, String, String),

    Window
}*/

#[derive(Debug, Clone)]
pub struct Function {
    pub name : String,
    pub args : Vec<SqlType>,
    pub ret : SqlType,
    pub doc : Option<String>,
    pub var_arg : bool,
}

/// A generic aggregate, with arbitrary initial, state and final functions
#[derive(Debug, Clone)]
pub struct Aggregate {
    pub name : String,
    pub init_func : String,
    pub state_func : String,
    pub final_func : String
}

/*/// ColAggregate are plain Rust functions that work with column-packed data.
/// They are the only type admissible for column-oriented engines, and can serve
/// as final functions for row-oriented engines (the init state is assumed to be an
/// empty vector, and the transition is assumed to be an append function).
pub struct ColAggregate {

}*/

pub type SqlSymbol<'a, T> = Symbol<'a, unsafe extern fn(&Context)->rusqlite::Result<T,rusqlite::Error>>;

pub enum LoadedFunc<'a> {
    F64(SqlSymbol<'a, f64>),
    I32(SqlSymbol<'a, i32>),
    Text(SqlSymbol<'a, String>),
    Bytes(SqlSymbol<'a, Vec<u8>>)
}

#[derive(Debug, Clone)]
pub struct Job {
    pub name : String,
    pub doc : Option<String>
}

impl TryFrom<toml::Value> for Aggregate {

    type Error = ();

    fn try_from(val : toml::Value) -> Result<Self, ()> {
        match val {
            toml::Value::Table(ref tbl) => {
                match tbl.get("init") {
                    Some(toml::Value::String(init_func)) => {
                        let state_func = match tbl.get("state") {
                            Some(toml::Value::String(s)) => s.clone(),
                            _ => return Err(())
                        };
                        let final_func = match tbl.get("final") {
                            Some(toml::Value::String(s)) => s.clone(),
                            _ => return Err(())
                        };
                        let name = match tbl.get("name") {
                            Some(toml::Value::String(s)) => s.clone(),
                            _ => return Err(())
                        };
                        Ok(Self{ init_func : init_func.clone(), name, state_func, final_func })
                    },
                    _ => Err(())
                }
            },
            _ => Err(())
        }
    }

}

impl FromStr for Function {

    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let mut spl = s.trim().split("->");
        let left = spl.next().ok_or(())?;
        let right = spl.next().ok_or(())?;
        println!("left={}", left);
        println!("right={}", right);
        let mut spl_left = left.split('(');
        let name = spl_left.next().ok_or(())?.to_string();
        println!("name={}", name);
        let full_args = spl_left.next().ok_or(())?
            .split(')').next().ok_or(())?;
        println!("full args={}", full_args);
        println!("next = {:?}", spl_left.next());
        if spl_left.next().is_some() {
            return Err(());
        }
        let mut args : Vec<SqlType> = Vec::new();
        for s in full_args.split(',') {
            args.push(SqlType::try_from(s)?);
        }
        println!("args={:?}", args);
        let ret = SqlType::try_from(right)?;
        Ok(Function{
            name,
            args,
            ret,
            doc : None,
            // mode : FunctionMode::Simple,
            var_arg : false,
            // var_ret : false
        })
    }

}

fn simple_from_table(tbl : &toml::value::Table) -> Result<Function, ()> {
    let variadic = match tbl.get("variadic") {
        Some(toml::Value::Boolean(b)) => *b,
        None => false,
        _ => return Err(()),
    };
    match tbl.get("name") {
        Some(toml::Value::String(s)) => {
            let mut func : Function = s.parse()?;
            func.var_arg = variadic;
            Ok(func)
        },
        _ => Err(())
    }
}

impl TryFrom<toml::Value> for Function {

    type Error = ();

    fn try_from(val : toml::Value) -> Result<Self, ()> {
        match val {
            toml::Value::String(s) => s.parse(),
            _ => Err(())
        }
    }
}

impl TryFrom<ItemFn> for Function {

    type Error = String;

    fn try_from(item_fn : ItemFn) -> Result<Self, String> {
        match item_fn.vis {
            Visibility::Public(_) => {
                parser::function_signature(item_fn)
            },
            _ => Err(format!("Function does not have public visibility"))
        }
    }

}

#[derive(Debug, Clone)]
pub enum ParseError {
    Arg(usize, String),
    Ret(usize, String),
    Attr,
    VarArg(usize),
    VarRet(usize),
    Other
}

impl fmt::Display for ParseError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ParseError::Arg(pos, which) => {
                format!("Argument at position {} is a {}, which is not a valid SQL type", pos, which)
            },
            ParseError::Ret(pos, which) => {
                format!("Return value at position {} is a {}, which is not a valid SQL type", pos, which)
            },
            ParseError::Attr => {
                format!("Function does not have #[sql], #[sql_agg] or #[sql_win] attribute")
            },
            ParseError::VarArg(pos) => {
                format!("Variadic argument at non-final position {}", pos)
            },
            ParseError::VarRet(pos) => {
                format!("Variadic return at non-final position {}", pos)
            },
            ParseError::Other => {
                format!("Failed at parsing function")
            }
        };
        write!(f, "{}", msg)
    }

}

impl Error for ParseError { }

/*impl TryFrom<String> for FunctionMode {

    type Error = ();

    fn try_from(s : String) -> Result<FunctionMode, ()> {
        match &s[..] {
            "Simple" => Ok(FunctionMode::Simple),
            "Aggregate" => /*Ok(FunctionMode::Aggregate),*/ unimplemented!(),
            "Window" => Ok(FunctionMode::Window),
            _ => Err(())
        }
    }

}

impl fmt::Display for FunctionMode {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            FunctionMode::Simple => "Simple",
            FunctionMode::Aggregate(_,_,_) => unimplemented!(),
            FunctionMode::Window => "Window"
        };
        write!(f, "{}", name)
    }

}*/

/// Gets type and return whether the type was nested into Vec<Vec<T>>
/// Return type string if type could not be converted into Sqltype.
fn get_simple_type(ts : TokenStream) -> Result<(SqlType, bool), String> {
    let ty_str : String = format!("{}", ts)
        .chars().filter(|c| !c.is_whitespace()).collect();
    let res_agg_ty : Result<SqlAggType,_> = (&ty_str[..]).try_into();
    match res_agg_ty {
        Ok(agg_ty) => {
            let nested = match agg_ty {
                SqlAggType::Nested(_) => true,
                SqlAggType::Simple(_) | SqlAggType::Owned(_) => false
            };
            Ok((agg_ty.inner(), nested))
        },
        Err(_) => {
            let simpl : SqlType = (&ty_str[..]).try_into()
                .map_err(|_| ty_str)?;
            Ok((simpl, false))
        }
    }
}

pub fn search_mod(mod_name : &str, item : &Item) -> Option<ItemMod> {
    match item {
        Item::Mod(item_mod) => {
            if &item_mod.ident.to_string()[..] == mod_name {
                Some(item_mod.clone())
            } else {
                if let Some((_, items)) = &item_mod.content {
                    for item in items {
                        if let Some(m) = search_mod(mod_name, &item) {
                            return Some(m);
                        }
                    }
                }
                None
            }
        },
        _ => None
    }
}

pub fn read_funcs_at_toplevel(items : &[Item]) -> Result<Vec<Function>, String> {
    let mut funcs = Vec::new();
    for item in items {
        match item {
            Item::Fn(item_fn) => {
                match Function::try_from(item_fn.clone()) {
                    Ok(f) => funcs.push(f),
                    Err(e) => { println!("{}", e); return Err(e); }
                }
            },
            _ => { }
        }
    }
    Ok(funcs)
}

/// Try to parse all publicly-exported functions from the informed module
/// as SQL functions.
pub fn read_funcs_at_item(item_mod : ItemMod) -> Result<Vec<Function>, String> {
    if let Some((_, items)) = item_mod.content {
        read_funcs_at_toplevel(&items[..])
    } else {
        Ok(Vec::new())
    }
}

fn search_doc_at_item(item : Item, docs : &mut HashMap<String, Option<String>>) {
    match item {
        Item::Mod(item_mod) => {
            match item_mod.vis {
                Visibility::Public(_) => {
                    println!("Searching doc at mod: {:?}", item_mod);
                    if let Some((_, items)) = item_mod.content {
                        for item in items {
                            println!("Searching doc at item: {:?}", item);
                            search_doc_at_item(item, docs);
                            /*if let Some(doc) =  {
                                return Some(doc);
                            }*/
                        }
                    }
                },
                _ => { }
            }
        },
        Item::Fn(item_fn) => {
            match item_fn.vis {
                Visibility::Public(_) => {
                    println!("Found function at source: {}", item_fn.sig.ident.to_string());
                    let fn_name = item_fn.sig.ident.to_string();
                    if let Some(mut val) = docs.get_mut(&fn_name) {
                        if val.is_none() {
                            *val = parser::load_doc(&item_fn);
                        }
                    }
                }
                _ => { }
            }
        },
        _ => { }
    }
}

/// Searches the source tree for the documentation of the function named f.
/// If f is not found or does not have any documentation, returns none.
fn search_doc_at_tree(content : &str, docs : &mut HashMap<String, Option<String>>) {
    let res_content : Result<syn::File,_> = syn::parse_str(content);
    if let Ok(t) = res_content {
        println!("Parsed file: {:?}", t);
        for item in t.items {
            search_doc_at_item(item, docs);
            /*if let Some(doc) =
                return Some(doc);
            }*/
        }
    }
    //None
}

pub fn search_doc_at_dir(dir : &Path, docs : &mut HashMap<String, Option<String>>) {
    if let Ok(dir_content) = dir.read_dir() {
        for entry in dir_content {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    println!("Found directory: {:?}", entry);
                    search_doc_at_dir(&entry.path(), docs);
                    //if let Some(doc) =  {
                    //    return Some(doc);
                    //}
                } else {
                    if entry.path().extension().and_then(|e| e.to_str()) == Some("rs") {
                        println!("Found rs file: {:?}", entry);
                        let mut content = String::new();
                        if let Ok(mut file) = fs::File::open(&entry.path()) {
                            file.read_to_string(&mut content);
                            search_doc_at_tree(&content, docs);
                        } else {
                            println!("Error reading file at {:?}", entry);
                        }
                        //if let Some(doc) =  {
                        //    return Some(doc);
                        //}
                    }
                }
            }
        }
    }
}

/*/// Returns function mode and doc attributes
pub fn get_attributes(f : &ItemFn) -> Result<(FunctionMode, Option<String>), ParseError> {
    let mut opt_mode : Option<FunctionMode> = None;
    let mut doc_content = String::new();
    for attr in &f.attrs {
        let ident = attr.path.get_ident().to_token_stream().to_string();
        println!("attr identity: {}", ident);
        match &ident[..] {
            "doc" => doc_content += &attr.tokens.to_string()[..],
            "sql" => if opt_mode.is_none() {
                opt_mode = Some(FunctionMode::Simple);
            } else {
                return Err(ParseError::Attr);
            },
            "sql_agg" => if opt_mode.is_none() {
                opt_mode = /*Some(FunctionMode::Aggregate);*/ unimplemented!()
            } else {
                return Err(ParseError::Attr);
            },
            "sql_win" => if opt_mode.is_none() {
                opt_mode = Some(FunctionMode::Window);
            } else {
                return Err(ParseError::Attr);
            }
            _ => { }
        }
    }
    let mode = if let Some(mode) = opt_mode {
        mode
    } else {
        return Err(ParseError::Attr);
    };
    doc_content = doc_content.clone().chars()
        .filter(|c| *c != '"' && *c != '=')
        .collect();
    doc_content = doc_content.clone().trim_matches(' ').to_string();
    let doc = if doc_content.is_empty() { None } else { Some(doc_content) };
    Ok((mode, doc))
}*/

/*/// Parse arguments, and if successful tell whether the function has
/// a variadic last argument.
pub fn parse_agg_arguments(f : &ItemFn) -> Result<(Vec<SqlType>, bool), ParseError> {
    let mut args : Vec<SqlType> = Vec::new();
    let inputs = f.sig.inputs.iter();
    let mut var_arg = false;
    let n_arg = inputs.clone().count();
    for (i, input) in inputs.enumerate() {
        match input {
            FnArg::Typed(typed) => {
                let (ty, nested) = get_simple_type(typed.ty.to_token_stream())
                    .map_err(|e| ParseError::Arg(i, e) )?;
                args.push(ty);
                if nested {
                    if i == n_arg - 1 {
                        var_arg = true;
                    } else {
                        return Err(ParseError::VarArg(i));
                    }
                }
            },
            _ => {  }
        }
    }
    Ok((args, var_arg))
}*/

/*/// Parse arguments, and if successful tell whether the function has
/// a variadic last argument.
pub fn parse_agg_return(f : &ItemFn) -> Result<(Vec<SqlType>, bool), ParseError> {
    let mut ret : Vec<SqlType> = Vec::new();
    let mut var_ret = false;
    match &f.sig.output {
        ReturnType::Type(_, ref bx_type) => {
            match bx_type.as_ref() {
                syn::Type::Tuple(tuple) => {
                    let n_ret = tuple.elems.iter().count();
                    for (i, t) in tuple.elems.iter().enumerate() {
                        let (ty, nested) = get_simple_type(t.to_token_stream())
                            .map_err(|e| ParseError::Ret(i, e) )?;
                        ret.push(ty);
                        if nested {
                            if i == n_ret - 1 {
                                var_ret = true;
                            } else {
                                return Err(ParseError::VarRet(i));
                            }
                        }
                    }
                },
                ty => {
                    let (ty, nested) = get_simple_type(ty.to_token_stream())
                        .map_err(|e| ParseError::Ret(0, ty.to_token_stream().to_string()) )?;
                    ret.push(ty);
                    var_ret = nested;
                }
            }
        },
        _ => return Err(ParseError::Other)
    }
    Ok((ret, var_ret))
}*/

/*/// Returns (name, arg types, return type)
pub fn agg_function_signature(f : ItemFn) -> Result<Function, ParseError> {
    let name = f.sig.ident.to_token_stream().to_string();
    let (args, var_arg) = parse_agg_arguments(&f)?;
    let (ret, var_ret) = parse_agg_return(&f)?;
    let (mode, doc) = get_attributes(&f)?;
    Ok( Function{ name, args, ret, doc, mode, var_arg, var_ret } )
}

/// Apply function_signature to a module and all its submodules recursively.
pub fn parse_mod_signature(
    item_mod : ItemMod
) -> Option<Vec<Function>> {
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
                        Visibility::Public(_) => { sigs.push(agg_function_signature(item_fn).ok()?); },
                        _ => { }
                    }
                },
                _ => { }
            }
        }
    }
    Some(sigs)
}

/// Takes any item and retrieves the signature of all functions,
/// running over modules recursively.
pub fn parse_fn_or_mod(
    item : Item
) -> Option<Vec<Function>> {
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
                Visibility::Public(_) => sigs.push(agg_function_signature(item_fn).ok()?),
                _ => { }
            }
        },
        _ => { }
    }
    Some(sigs)
}

pub fn parse_top_level_funcs(
    content : &str
) -> Option<Vec<Function>> {
    let t : File = syn::parse_str(content).ok()?;
    let mut sigs = Vec::new();
    for item in t.items {
        match item {
            Item::Fn(item_fn) => {
                match item_fn.vis {
                    Visibility::Public(_) => sigs.push(agg_function_signature(item_fn).ok()?),
                    _ => { },
                }
            },
            _ => { }
        }
    }
    Some(sigs)
}

/// Parse a full source file applying parse_fn_or_mod.
/// Return function name; arguments; return.
pub fn parse_nested_signatures(
    content : &str
) -> Option<Vec<Function>> {
    let t : File = syn::parse_str(content).ok()?;
    let mut sigs = Vec::new();
    for item in t.items {
        sigs.extend(parse_fn_or_mod(item)?);
    }
    Some(sigs)
}*/

/*#[test]
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
}*/

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

pub fn search_sources_in_crate(path : Path) -> Vec<Path> {
    let mut sources = Vec::new();
    if p.path().is_dir() {
        let paths = fs::read_dir(dir).unwrap();
        for p in paths {
            if p.path().is_dir() {
                sources.extend(search_rust_crates(p.path()));
            }
            if p.path().extension() == Some(OsStr::new("rs")) {
                sources.push(p);
            }
        }
    } else {
        println!("Tried to search for sources in non-dir path");
        return Vec::new();
    }
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


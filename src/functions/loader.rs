use libloading;
use crate::tables::table::{Table, Columns};
use libloading::{Library, Symbol};
use std::env;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::Read;
//use super::parser::{self, *};
use std::convert::TryInto;
use rusqlite::types::ToSql;
use ::queries::column::*;
use ::queries::*;
use crate::tables::table::*;

// use std::rc::Rc;
// use std::cell::RefCell;
// (1) Examine sources and update database with library/function names
// (2) Call libloading to load all functions that were parsed.
/*pub type TableFunc<'a> = Symbol<'a, unsafe extern fn(Columns, &[&str])->Result<Table,String>>;*/

#[derive(Debug)]
pub struct FunctionLoader {
    conn : Connection,
    libs : Vec<FunctionLibrary>
}

#[derive(Debug)]
pub enum FunctionErr {
    NotFound(Table),
    TypeMismatch(usize),
    TableAgg(String),
    UserErr(String)
}

impl FunctionLoader {

    /// Returns (source path, parsed_functions) if successful.
    fn parse_lib_source(path : &Path) -> Result<(String, Vec<Function>), String> {
        let mut fname = path.file_stem().ok_or(format!("Could not retrieve name"))?.to_str().unwrap().to_string();
        if fname.starts_with("lib") {
            fname = fname[3..].to_string();
        };
        let parent = path.parent().ok_or(String::from("Path does not have parent"))?;
        let parent2 = parent.parent().ok_or(String::from("Path does not have parent/parent"))?;
        let parent3 = parent2.parent().ok_or(String::from("Path does not have parent/parent"))?;
        let mut src_folder = parent2.to_path_buf();
        let mut src_folder2 = parent3.to_path_buf();
        src_folder.push("src");
        src_folder2.push("src");
        let mut src_cand_1 = parent.to_path_buf();
        src_cand_1.push(format!("{}.rs", &fname));
        let mut src_cand_2 = src_folder.clone();
        src_cand_2.push(format!("{}.rs", &fname));
        let mut src_cand_3 = src_folder.clone();
        src_cand_3.push(format!("lib.rs"));
        let mut src_cand_4 = src_folder2.clone();
        src_cand_4.push(format!("{}.rs", &fname));
        let mut src_cand_5 = src_folder2.clone();
        src_cand_5.push(format!("lib.rs"));
        let src_candidates = [
            src_cand_1.as_path(),
            src_cand_2.as_path(),
            src_cand_3.as_path(),
            src_cand_4.as_path(),
            src_cand_5.as_path()
        ];
        println!("Candidate source folders: {:?}", src_candidates);
        for cand in src_candidates.iter() {
            if let Ok(mut f) = File::open(cand) {
                println!("Chosen path: {:?}", cand);
                let mut src = String::new();
                f.read_to_string(&mut src).map_err(|e| format!("{}", e))?;
                let funcs = parse_top_level_funcs(&src).ok_or(String::from("Error parsing functions"))?;
                println!("Parsed functions: {:?}", funcs);
                let src_path = cand.to_str().ok_or(format!("Could not convert path to string"))?.to_string();
                return Ok((src_path, funcs))
            }
        }
        Err(String::from("Source file not found"))
    }

    fn insert_types(conn : &Connection, func_id : i64, tbl : &str, types : &[SqlType]) -> Result<(), String> {
        let mut stmt = conn.prepare(&format!("insert into {} (fn_id, pos, type)
            values (?1, ?2, ?3);", tbl)[..]).unwrap();
        for (i, ty) in types.iter().enumerate() {
            let ty_str = format!("{}", ty);
            stmt.execute(&[&func_id as &dyn ToSql, &(i as i32) as &dyn ToSql, &ty_str as &dyn ToSql])
                .map_err(|e| format!("{}", e) )?;
        }
        Ok(())
    }

    fn insert_functions(&mut self, id : i64, funcs : &[Function]) -> Result<(), String> {
        let mut stmt_func = self.conn.prepare("insert into function \
            (lib_id, name, doc, fn_mode, var_arg, var_ret) \
            values (?1, ?2, ?3, ?4, ?5, ?6);").unwrap();
        for f in funcs.iter() {
            stmt_func.execute(&[
                &id as &dyn ToSql,
                &f.name as &dyn ToSql,
                &f.doc as &dyn ToSql,
                &f.mode.to_string() as &dyn ToSql,
                &f.var_arg as &dyn ToSql,
                &f.var_ret as &dyn ToSql
            ]).map_err(|e| format!("{}", e))?;
            let func_id = self.conn.last_insert_rowid();
            Self::insert_types(&self.conn, func_id, "arg", &f.args[..])?;
            Self::insert_types(&self.conn, func_id, "ret", &f.ret[..])?;
        }
        Ok(())
    }

    /// Returns the number of recovered functions if successful, or an error
    /// message if unsucessful.
    pub fn add_crate(&mut self, path_str : &str) -> Result<usize, String> {
        let path = Path::new(path_str);
        let fname = path.file_stem().ok_or(format!("Could not retrieve name"))?
            .to_str().unwrap().to_string();
        let lib_name = if fname.starts_with("lib") {
            fname[3..].to_string()
        } else {
            fname.to_string()
        };
        self.remove_crate(&lib_name)?;
        let (src, funcs) = Self::parse_lib_source(path)?;
        let id = {
            let mut stmt = self.conn.prepare("insert into library (name, libpath, srcpath, active) values (?1, ?2, ?3, 0);")
                .map_err(|e| format!("{}", e) )?;
            stmt.execute(&[lib_name.clone(), path_str.into(), src])
                .map_err(|e| format!("{}", e) )?;
            self.conn.last_insert_rowid()
        };
        //if let Some(id) = opt_id {
        let cloned_funcs = funcs.clone();
        self.insert_functions(id, &cloned_funcs[..])?;
        let n = funcs.len();
        let lib = Library::new(path_str).map_err(|e| format!("Library not found: {}", e) )?;
        self.libs.push(FunctionLibrary {
            funcs,
            local_path : path_str.to_string(),
            remote_path : None,
            name : lib_name.to_string(),
            lib,
            active : false
        });
        Ok(n)
        //} else {
        //    Err(String::from("Could not retrieve inserted library id"))
        //}
    }

    pub fn set_active_status(&mut self, name : &str, status : bool) -> Result<(), String> {
        let status_code : i32 = if status { 1 } else { 0 };
        let mut stmt = self.conn.prepare("update library set active = ?1 where name = ?2;").unwrap();
        stmt.execute(&[&status_code as &dyn ToSql, &name as &dyn ToSql]).map_err(|e| format!("{}", e))?;
        Ok(())
    }

    pub fn remove_crate(&mut self, name : &str) -> Result<(), String> {
        if let Some(ix) = self.libs.iter().position(|lib| lib.name == name ) {
            let mut stmt = self.conn.prepare("delete from library where name = ?1;").unwrap();
            stmt.execute(&[name]).map_err(|e| format!("{}", e) )?;
            self.libs.remove(ix);
        }
        Ok(())
    }

    fn update_full_registry(&mut self) {
        let lib_names : Vec<String> = self.libs.iter()
            .map(|lib| lib.name.clone() )
            .collect();
        for name in lib_names.iter() {
            self.update_crate(&name[..]);
        }
    }

    fn update_crate(&mut self, name : &str) -> Result<(), String> {
        if let Some(lib) = self.libs.iter().find(|lib| lib.name == name ) {
            let path = lib.local_path.clone();
            self.remove_crate(&lib.name.clone()[..]);
            self.add_crate(&path[..]);
            Ok(())
        } else {
            Err(format!("Library {} not found", name))
        }
    }

    pub fn load() -> Result<Self, &'static str> {
        let exe_path = env::current_exe().map_err(|_| "Could not get executable path")?;
        let exe_dir = exe_path.as_path().parent().ok_or("CLI executable has no parent dir")?
            .to_str().ok_or("Could not convert path to str")?;
        let registry_path = String::from(exe_dir) + "/../../registry/registry.db";
        let conn = Connection::open(&registry_path)
            .map_err(|e| { println!("{}", e); "Could not open registry database" })?;
        let libs = Vec::new();
        let mut reg = Self{ conn, libs };
        reg.reload_libs()?;
        Ok(reg)
    }

    pub fn reload_libs(&mut self) -> Result<(), &'static str> {
        let new_libs : Vec<FunctionLibrary> = Self::read_libs(&self.conn)?;
        self.libs.clear();
        self.libs.extend(new_libs);
        Ok(())
    }

    fn read_libs(conn : &Connection) -> Result<Vec<FunctionLibrary>, &'static str> {
        let mut stmt = conn.prepare("select name, libpath, active from library;").unwrap();
        let mut ans = stmt.query(rusqlite::NO_PARAMS).unwrap();
        let mut libs : Vec<FunctionLibrary> = Vec::new();
        while let Ok(opt_row) = ans.next() {
            if let Some(r) = opt_row {
                let name : String = r.get(0).unwrap();
                let path : String = r.get(1).unwrap();
                let active_code : i32 = r.get(2).unwrap();
                let active = if active_code == 1 { true } else { false };
                let lib = Library::new(path.clone()).map_err(|_| "Library not found")?;
                //let funcs = FunctionLibrary::load_functions(&conn, &name[..], &libr)?;
                let mut lib = FunctionLibrary {
                    funcs : Vec::new(),
                    local_path : path,
                    remote_path : None,
                    name : name,
                    lib,
                    active
                };
                lib.reload_functions(&conn);
                libs.push(lib);
            } else {
                break;
            }
            println!("Here");
        }
        Ok(libs)
    }

    pub fn lib_list<'a>(&'a self) -> Vec<&'a FunctionLibrary> {
        self.libs.iter().collect()
    }

    pub fn fn_list_for_lib<'a>(&'a self, lib_name : &'a str) -> Vec<&'a Function> {
        let mut funcs = Vec::new();
        if let Some(lib) = self.libs.iter().find(|lib| &lib.name[..] == lib_name ) {
            funcs.extend(lib.funcs.iter().map(|f| f));
        }
        funcs
    }

    pub fn find_fn<'a>(&'a self, fn_name : &'a str) -> Option<(&'a FunctionLibrary, &'a Function)> {
        for lib in self.libs.iter() {
            let fns = self.fn_list_for_lib(&lib.name[..]);
            if let Some(f) = fns.iter().find(|f| f.name == fn_name) {
                return Some((lib, f))
            }
        }
        None
    }

    fn call<'a>(f : &'a Function, lib : &'a Library, cols : Vec<Column>) -> Result<Vec<Column>, FunctionErr> {
        unsafe {
            let f : Symbol<'a, unsafe extern fn(Vec<Column>)->Result<Vec<Column>,String>> =
                lib.get(f.name.as_bytes()).map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
            let ans = f(cols).map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
            Ok(ans)
        }
    }

    pub fn try_exec_fn(
        &self,
        fn_name : String,
        arg_names : Vec<String>,
        tbl : Table
    ) -> Result<Table, FunctionErr> {
        if let Some((lib, f)) = self.find_fn(&fn_name[..]) {
            let cols = tbl.take_columns();
            let ans_cols = Self::call(&f, &lib.lib, cols)?;
            let mut col_names = f.get_col_names(arg_names)
                .map_err(|err_ix| FunctionErr::TableAgg(format!("Error retrieving name for column {}", err_ix)))?;
            let tbl = Table::new(col_names, ans_cols)
                .map_err(|e| FunctionErr::TableAgg(format!("{}", e)))?;
            Ok(tbl)
        } else {
            Err(FunctionErr::NotFound(tbl))
        }
    }

    pub fn get_func<'a>(&'a self, name : &str) -> Option<&'a Function> {
        for lib in self.libs.iter() {
            if let Some(func) = lib.funcs.iter().find(|func| func.name == name) {
                return Some(func)
            }
        }
        return None
    }

    pub fn has_func_name(&self, name : &str) -> bool {
        self.get_func(name).is_some()
    }

    pub fn get_doc(&self, name : &str) -> Option<String> {
        self.get_func(name).map(|func| func.doc.clone().unwrap_or(String::new()) )
    }

    pub fn get_args(&self, name : &str) -> Option<Vec<String>> {
        self.get_func(name).map(|func| {
            func.args.iter().map(|a| format!("{}", a)).collect::<Vec<_>>()
        })
    }

    /*pub fn retrieve_func(&self, name : &str) -> Option<TableFunc> {
        for lib in self.libs.iter() {
            if lib.funcs.iter().any(|func| func.name == name) {
                match lib.retrieve_single(name) {
                    Ok(symbol) => { return Some(symbol) },
                    Err(e) => { println!("{}", e); return None; }
                }
            }
        }
        None
    }*/
}

#[derive(Debug)]
pub struct FunctionLibrary {
    pub name : String,
    pub active : bool,
    funcs : Vec<Function>,
    local_path : String,
    remote_path : Option<String>,
    lib : Library
}

impl FunctionLibrary {

    /*pub fn retrieve_all<'a>(
        &'a self
    ) -> Result<HashMap<&'a str, TableFunc<'a>>, &'static str> {
        let mut func_ptrs = HashMap::new();
        for func in &self.funcs {
            let func_ptr = self.retrieve_single(&func.name[..])?;
            func_ptrs.insert(&func.name[..], func_ptr);
        }
        Ok(func_ptrs)
    }

    pub fn retrieve_single<'a>(
        &'a self,
        name : &str
    ) -> Result<TableFunc<'a>, &'static str> {
        if !self.funcs.iter().any(|func| func.name == name) {
            return Err("No function with the given name");
        }
        unsafe {
            let func_ptr : TableFunc<'a> = self.lib.get(name.as_bytes())
                .map_err(|_| { println!("Error loading {}", name); "Could not load function" })?;
            Ok(func_ptr)
        }
    }*/

    fn get_type_info(conn : &Connection, func_id : i64, tbl_name : &str) -> Vec<SqlType> {
        let mut types = Vec::new();
        let mut stmt = conn.prepare(&format!("select type \
            from {} inner join function on {}.fn_id = function.id \
            where id = ?1 order by pos;", tbl_name, tbl_name)[..]).unwrap();
        let ans = stmt.query(&[&func_id]);
        if let Ok(mut res_args) = ans {
            while let Ok(opt_row) = res_args.next() {
                match opt_row {
                    Some(row) => {
                        let type_str : String = row.get(0).unwrap();
                        let ty : SqlType = (&type_str[..]).try_into().unwrap();
                        types.push(ty);
                    },
                    None => {
                        break;
                    }
                }
            }
        }
        types
    }

    pub fn reload_functions(&mut self, conn : &Connection) -> Result<(), &'static str> {
        self.funcs.clear();
        let mut fn_stmt = conn.prepare(
            "select id, name, doc, \
            fn_mode, var_arg, var_ret from function inner join library \
            on library.id = function.lib_id \
            where library.name = ?1;"
        ).unwrap();
        let fn_result = fn_stmt.query_map(&[&self.name], |row| {
            let id = row.get::<usize, i64>(0)?;
            let name = row.get::<usize, String>(1)?;
            let doc = row.get::<usize, String>(2)?;
            let mode = row.get::<usize, String>(3)?;
            let var_arg = row.get::<usize, bool>(4)?;
            let var_ret = row.get::<usize, bool>(5)?;
            Ok((id, name, doc, mode, var_arg, var_ret))
        });
        if let Ok(res_libs) = fn_result {
            let mut funcs = Vec::new();
            for fn_row in res_libs {
                if let Ok(fn_data) = fn_row {
                    let func_id : i64 = fn_data.0;
                    let name : String = fn_data.1;
                    let doc : String = fn_data.2;
                    let mode : FunctionMode = fn_data.3.try_into()
                        .map_err(|_| "Could not pase function mode")?;
                    let var_arg = fn_data.4;
                    let var_ret = fn_data.5;
                    let args = Self::get_type_info(&conn, func_id, "args");
                    let ret = Self::get_type_info(&conn, func_id, "ret");
                        funcs.push(Function {name, args, doc : Some(doc), ret, mode, var_arg, var_ret });
                } else {
                    return Err("Error retrieving nth row");
                }
            }
            self.funcs.extend(funcs);
            Ok(())
        } else {
            Err("Error mapping argument query results")
        }
    }

}

/*#[derive(Clone, Debug)]
pub struct ClientFunction {
    pub name : String,
    pub args : Vec<String>,
    pub doc : String,
    // func : fn(&Table, Vec<String>)->Result<Table,String>
    // func : Symbol<'a, unsafe extern fn(Table, Vec<String>)->Result<Table,String>>
}

#[derive(Clone, Debug)]
pub struct FunctionCall {
    pub name : String,
    pub args : Vec<String>,
    pub source : Vec<usize>,
    //dst : Vec<String>
}

impl FunctionCall {

    pub fn new(call : (String, Vec<String>), selected : Vec<usize>) -> Self {
        Self{
            name : call.0,
            args : call.1,
            source : selected,
        }
    }

}*/

/*impl ClientFunction {
    /*pub fn set_arg(&mut self, arg : String, val : String) -> Result<(), String> {
        let arg_pos = self.args.iter().position(|a| a.0 == arg )
            .ok_or(String::from("Argument not found"))?;
        if let Some(mut arg) = self.args.get_mut(arg_pos) {
            arg.1 = Some(val)
        }
        Ok(())
    }
    pub fn run(&self, tbl : &Table) -> Result<Table, String> {
        let mut valid_args = Vec::new();
        for arg in self.args.iter() {
            valid_args.push(
                arg.1.ok_or(String::from("Not all arguments are valid"))?.clone()
            );
        };
        (self.func)(tbl, valid_args)
    }*/
}*/

/*pub struct FunctionCall {
    source : String,
    func : ClientFunction,
    dst : String
}
pub struct NumericScript {
    registry : ClientFunctionRegistry,
    source : String,
    procedures : FunctionCall,
    dest : String
}*/

// ClientFunctionRegistry::load("/home/diego/Software/mvlearn-sqlite/target/debug/libmvlearn.so")




use libloading;
use crate::tables::table::{Table, Columns};
use libloading::{Library, Symbol};
use std::env;
use rusqlite::Connection;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

// (1) Examine sources and update database with library/function names
// (2) Call libloading to load all functions that were parsed.

pub type TableFunc<'a> = Symbol<'a, unsafe extern fn(Columns, &[&str])->Result<Table,String>>;

#[derive(Debug)]
pub struct NumRegistry {
    conn : Connection,
    libs : Vec<NumFunctionLibrary>
}

impl NumRegistry {

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
        reg.reload_all_funcs()?;
        Ok(reg)
    }

    pub fn reload_all_funcs(&mut self) -> Result<(), &'static str> {
        for lib in self.libs.iter_mut() {
            lib.repopulate(&self.conn)?;
        }
        Ok(())
    }

    pub fn reload_libs(&mut self) -> Result<(), &'static str> {
        let new_libs : Vec<NumFunctionLibrary> = Self::load_libs(&self.conn)?;
        self.libs.clear();
        self.libs.extend(new_libs);
        Ok(())
    }

    pub fn load_libs(conn : &Connection) -> Result<Vec<NumFunctionLibrary>, &'static str> {
        let mut stmt = conn.prepare("select name, libpath from library;").unwrap();
        let mut ans = stmt.query(rusqlite::NO_PARAMS).unwrap();
        let mut libs : Vec<NumFunctionLibrary> = Vec::new();
        while let Ok(opt_row) = ans.next() {
            if let Some(r) = opt_row {
                let name : String = r.get(0).unwrap();
                let path : String = r.get(1).unwrap();
                let lib = Library::new(path.clone()).map_err(|_| "Library not found")?;
                //let funcs = NumFunctionLibrary::load_functions(&conn, &name[..], &libr)?;
                let lib = NumFunctionLibrary {
                    funcs : Vec::new(),
                    local_path : path,
                    remote_path : None,
                    name : name,
                    lib
                };
                libs.push(lib);
            } else {
                break;
            }
            println!("Here");
        }

        //for lib in libs.iter_mut() {
        //    lib.reload_funcs(&conn)?;
        //}
        Ok(libs)
        //if libs.len() >= 1 {
            //Err("What")
            //Ok(libs)
        //} else {
        //    Err("No libraries available")
        //}
    }

    pub fn function_list<'a>(&'a self) -> Vec<&'a NumFunction> {
        let mut funcs = Vec::new();
        for lib in &self.libs {
            funcs.extend(lib.funcs.iter().map(|f| f));
        }
        funcs
    }

    pub fn get_func<'a>(&'a self, name : &str) -> Option<&'a NumFunction> {
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
        self.get_func(name).map(|func| func.doc.clone())
    }

    pub fn get_args(&self, name : &str) -> Option<Vec<String>> {
        self.get_func(name).map(|func| func.args.clone())
    }

    pub fn retrieve_func(&self, name : &str) -> Option<TableFunc> {
        for lib in self.libs.iter() {
            if lib.funcs.iter().any(|func| func.name == name) {
                match lib.retrieve_single(name) {
                    Ok(symbol) => { return Some(symbol) },
                    Err(e) => { println!("{}", e); return None; }
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct NumFunctionLibrary {
    funcs : Vec<NumFunction>,
    local_path : String,
    remote_path : Option<String>,
    name : String,
    lib : Library
}

impl NumFunctionLibrary {

    pub fn retrieve_all<'a>(
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
    }

    pub fn repopulate(&mut self, conn : &Connection) -> Result<(), &'static str> {
        self.funcs.clear();
        let mut fn_stmt = conn.prepare(
            &format!("select function.id, function.name, function.doc \
            from function inner join library on library.id = function.lib_id
            where library.name = '{}';", &self.name)
        ).unwrap();
        let fn_result = fn_stmt.query_map(rusqlite::NO_PARAMS, |row| {
            let id = row.get::<usize, i32>(0)?;
            let name = row.get::<usize, String>(1)?;
            let doc = row.get::<usize, String>(2)?;
            Ok((id, name, doc))
        });
        if let Ok(res_libs) = fn_result {
            let mut funcs = Vec::new();
            for fn_row in res_libs {
                if let Ok(fn_data) = fn_row {
                    let func_id : i32 = fn_data.0;
                    let name : String = fn_data.1;
                    let doc : String = fn_data.2;
                    let mut args = Vec::new();
                    let mut arg_stmt = conn.prepare("select argument.name \
                        from argument inner join function on argument.fn_id = function.id \
                        where function.id = ?1 order by argument.pos;").unwrap();
                    let ans = arg_stmt.query(&[&func_id]);
                    if let Ok(mut res_args) = ans {
                        while let Ok(opt_row) = res_args.next() {
                            match opt_row {
                                Some(row) => {
                                    let arg_name : String = row.get(0).unwrap();
                                    args.push(arg_name);
                                },
                                None => {
                                    break;
                                }
                            }
                        }
                        funcs.push(NumFunction {name, args, doc });
                    } else {
                        return Err("Error retrieving nth row");
                    }
                }
            }
            self.funcs.extend(funcs);
            Ok(())
        } else {
            Err("Error mapping argument query results")
        }
    }

}

#[derive(Clone, Debug)]
pub struct NumFunction {
    pub name : String,
    pub args : Vec<String>,
    pub doc : String,
    // func : fn(&Table, Vec<String>)->Result<Table,String>
    // func : Symbol<'a, unsafe extern fn(Table, Vec<String>)->Result<Table,String>>
}

/*impl NumFunction {
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
    func : NumFunction,
    dst : String
}
pub struct NumericScript {
    registry : NumFunctionRegistry,
    source : String,
    procedures : FunctionCall,
    dest : String
}*/

// NumFunctionRegistry::load("/home/diego/Software/mvlearn-sqlite/target/debug/libmvlearn.so")




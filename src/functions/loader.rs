use libloading;
use crate::tables::table::{Table, Columns};
use libloading::{Library, Symbol};
use std::env;
use rusqlite::{Connection, config::DbConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Read;
use std::convert::TryInto;
use rusqlite::types::ToSql;
use crate::tables::column::*;
use super::sql_type::*;
use super::function::{self, *};
use crate::tables::table::*;
use toml;
use std::convert::TryFrom;
use super::function::*;
use super::parser;

// use std::rc::Rc;
// use std::cell::RefCell;
// (1) Examine sources and update database with library/function names
// (2) Call libloading to load all functions that were parsed.
// *pub type TableFunc<'a> = Symbol<'a, unsafe extern fn(Columns, &[&str])->Result<Table,String>>;*/

/// Structure that carries the state of which user-defined functions (UDFs)
/// are loaded in the user session. Its methods access the filesystem to verify the validity of a
/// crate as an exporter of UDFs, add paths to dynamic libraries that will be loaded later on,
/// and add/remove libraries from the registry and active and de-active them.
/// The registry is a SQLite database shipped with queries,
/// and this structure keeps a connection to this database alive to perform those operations.
/// The ownership of the loader is shared by the FunctionRegistry GUI and the EnvironmentSource.
/// The first item offers the user interface to add/remove functions; the last item links the
/// active user-defined functinos for each new local connection.
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

    fn search_lib_path(root : &Path, pkg_name : &str) -> Option<String> {
        let root_buf = root.to_path_buf();
        let fname = format!("lib{}.so", pkg_name);

        let mut tgt_release = root_buf.clone();
        tgt_release.push("target");
        tgt_release.push("release");
        tgt_release.push(fname.clone());

        let mut tgt_debug = root_buf.clone();
        tgt_debug.push("target");
        tgt_debug.push("debug");
        tgt_debug.push(fname);

        for cand in [tgt_release, tgt_debug].iter() {
            println!("Searching lib at {:?}", cand);
            if cand.as_path().exists() {
                if let Some(path_str) = cand.as_path().to_str() {
                    println!("Compiled library found at {}", path_str);
                    return Some(path_str.to_string());
                }
            }
        }
        None
    }

    /// Tries to retrieve a sequence of functions either from the top-level module at
    /// the informed path (if module_name) is None, or from a submodule (if module has been
    /// informed).
    fn parse_sql_file(module_name : Option<&str>, path : &Path) -> Result<Vec<Function>, String> {
        let mut content = String::new();
        let mut f = File::open(path).unwrap();
        f.read_to_string(&mut content);
        let t : syn::File = syn::parse_str(&content).map_err(|e| format!("{}", e) )?;
        if let Some(module_name) = module_name {
            for item in t.items {
                match function::search_mod(&module_name, &item) {
                    Some(module) => { return function::read_funcs_at_item(module); },
                    None => { }
                }
            }
            Err(format!("Module {} not found at file {}", module_name, path.display()))
        } else {
            let t : syn::File = syn::parse_str(&content).map_err(|e| format!("{}", e) )?;
            function::read_funcs_at_toplevel(&t.items[..])
        }
    }

    fn parse_sql_module(opt_module_name : Option<&str>, src : &Path) -> Result<Vec<Function>, String> {
        let sources = parser::search_sources_in_crate(&src);
        println!("found_sources: {:?}", sources);
        if let Some(module_name) = opt_module_name {
            for src in sources {
                let parent_has_mod_name = src.parent()
                    .and_then(|p| p.file_name().and_then(|f| f.to_str())) == Some(module_name);
                let file_has_mod_name = src.file_name().and_then(|s| s.to_str()) == Some(module_name);
                if parent_has_mod_name || file_has_mod_name {
                    return Self::parse_sql_file(None, &src);
                } else {
                    if let Ok(funcs) = Self::parse_sql_file(Some(module_name), &src) {
                        return Ok(funcs);
                    }
                }
            }
            Err(format!("Module {} not found at crate", module_name))
        } else {
            // TODO read lib key from toml file and use that value instead of lib,
            // if user informed it.
            let opt_lib_src = sources.iter()
                .find(|p| p.as_path().file_name().and_then(|s| s.to_str()) == Some("lib.rs"));
            if let Some(src) = opt_lib_src.as_ref() {
                Self::parse_sql_file(None, &src)
            } else {
                Err(format!("Root .lib file not found"))
            }
        }
    }

    fn get_src_lib_path(v : &toml::Value, parent : &Path, name : &str) -> Result<(String, String), String> {
        let src_path = v.get("lib")
            .and_then(|lib| lib.get("path"))
            .and_then(|path| {
                if let toml::Value::String(path) = path {
                    Some(path.clone())
                } else {
                    None
                }
            }).unwrap_or("src/lib.rs".to_string());
        println!("Found source path: {}", src_path);
        let lib_path = Self::search_lib_path(parent, &name)
            .ok_or(format!("No .so file found at target directory. Is the crate compiled with crate-type=[\"cdylib\"]"))?;
        println!("Found library path: {}", lib_path);
        let src_path_str = src_path.as_str().to_string();
        Ok((src_path.to_string(), lib_path.to_string()))
    }

    fn parse_function_vec(
        func_vals : &[toml::Value],
        src_folder : &Path
    ) -> Result<(Vec<Function>, Vec<Aggregate>, Vec<Job>), String> {
        let mut funcs = Vec::new();
        let mut aggs = Vec::new();
        let mut jobs = Vec::new();
        let mut docs : HashMap<String, Option<String>> = HashMap::new();
        let parse_str = |val : &toml::Value| {
            let s = toml::Value::try_into::<'_, String>(val.clone()).ok();
            s
        };
        let parse_seq = |val : &toml::Value| {
            let vs = toml::Value::try_into::<'_, Vec<String>>(val.clone()).ok();
            vs
        };
        let parse_type = |val : &String| {
            let ty : Option<SqlType> = (&val[..]).try_into().ok();
            ty
        };
        for f in func_vals.iter() {
            let agg = f.get("aggregate").and_then(parse_str);
            let init_func = f.get("init").and_then(parse_str);
            let state_func = f.get("state").and_then(parse_str);
            let final_func = f.get("final").and_then(parse_str);
            match (agg, init_func, state_func, final_func) {
                (Some(name), Some(init_func), Some(state_func), Some(final_func)) => {
                    let agg = Aggregate { name, init_func, state_func, final_func };
                    println!("Found aggregate: {:?}", agg);
                    aggs.push(agg);
                },
                _ => {
                    let scalar = f.get("scalar").and_then(parse_str);
                    let args = f.get("args")
                        .and_then(parse_seq)
                        .and_then(|args| {
                            let types = args.iter().filter_map(parse_type)
                                .collect::<Vec<_>>();
                            if types.len() == args.len() {
                                Some(types)
                            } else {
                                None
                            }
                        });
                    let ret = f.get("ret")
                        .and_then(parse_str)
                        .as_ref()
                        .and_then(parse_type);
                    match (scalar, args, ret) {
                        (Some(name), Some(args), Some(ret)) => {
                            let var_arg = f.get("var_arg")
                                .and_then(|v| {
                                    let b = toml::Value::try_into::<bool>(v.clone()).ok();
                                    b
                                }).unwrap_or(false);
                            let mut func = Function{ name, args, ret, doc : None, var_arg };
                            println!("Found function: {:?}", func);
                            docs.insert(func.name.clone(), None);
                            funcs.push(func);
                        },
                        _ => {
                            let job_name = f.get("job").and_then(parse_str);
                            match job_name {
                                Some(name) => {
                                    docs.insert(name.clone(), None);
                                    jobs.push(Job{name, doc : None});
                                },
                                None => return Err(format!("Invalid SQL metadata entry : {}", f))
                            }
                        }
                    }
                }
            }
        }

        /// Load documentation
        search_doc_at_dir(src_folder, &mut docs);
        for func in funcs.iter_mut() {
            match docs.get(&func.name) {
                Some(Some(doc)) => func.doc = Some(doc.clone()),
                _ => { }
            }
        }
        for job in jobs.iter_mut() {
            match docs.get(&job.name) {
                Some(Some(doc)) => job.doc = Some(doc.clone()),
                _ => { }
            }
        }
        Ok((funcs, aggs, jobs))
    }

    /// Returns (name, source path, lib path, parsed_functions, parsed aggregates) if successful.
    fn parse_toml(
        path : &Path
    ) -> Result<(String, String, String, Vec<Function>, Vec<Aggregate>, Vec<Job>), String> {
        let parent = path.parent()
            .ok_or(String::from("Path does not have parent"))?;
        let mut src_folder = parent.to_path_buf();
        src_folder.push("src");
        let mut content = String::new();
        let mut f = File::open(&path)
            .map_err(|e| format!("Could not read toml file: {}", e) )?;
        f.read_to_string(&mut content);

        /// Retrieve package
        let v : toml::Value = content.parse()
            .map_err(|e| format!("Could not parse toml file: {}", e) )?;
        let pkg = if let Some(pkg) = v.get("package") {
            pkg
        } else {
            return Err(format!("Missing pkg entry"));
        };

        // Get library name, root source path, and library path
        let name = if let Some(name_value) = pkg.get("name") {
            if let toml::Value::String(s) = name_value {
                s.clone()
            } else {
                return Err(String::from("Package name field should be a string"));
            }
        } else {
            return Err(format!("Missing package name"));
        };
        let(src_path, lib_path) = Self::get_src_lib_path(&v, parent, &name[..])?;

        let sql_meta = pkg.get("metadata")
            .and_then(|meta| { println!("meta={:?}", meta); meta.get("sql") })
            .ok_or(String::from("Manifest missing 'sql' metadata field"))?;
        match sql_meta {
            toml::Value::String(s) => {
                let mod_name = if s.len() == 0 { None } else { Some(&s[..]) };
                let funcs = Self::parse_sql_module(mod_name, &src_folder)?;
                Ok((name.to_string(), src_path, lib_path, funcs, Vec::new(), Vec::new()))
            },
            toml::Value::Array(func_vals) => {
                if func_vals.len() == 0 {
                    let funcs = Self::parse_sql_module(None, &src_folder)?;
                    Ok((name.to_string(), src_path, lib_path, funcs, Vec::new(), Vec::new()))
                } else {
                    let (funcs, aggs, jobs) = Self::parse_function_vec(
                        &func_vals[..],
                        &src_folder.as_path()
                    )?;
                    Ok((name.to_string(), src_path, lib_path, funcs, aggs, jobs))
                }
            },
            _ => {
                Err(format!("sql metadata should be a string or table"))
            }
        }
    }

    /*fn parse_lib_source(path : &Path) -> Result<(String, String, Vec<Function>), String> {
        let mut fname = path.file_stem()
            .ok_or(format!("Could not retrieve name"))?.to_str().unwrap().to_string();
        if fname.starts_with("lib") {
            fname = fname[3..].to_string();
        };
        unimplemented!()
        /*let parent = path.parent().ok_or(String::from("Path does not have parent"))?;
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
        Err(String::from("Source file not found"))*/
    }*/

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
            (lib_id, name, doc, var_arg, ret) \
            values (?1, ?2, ?3, ?4, ?5);").unwrap();
        for f in funcs.iter() {
            stmt_func.execute(&[
                &id as &dyn ToSql,
                &f.name as &dyn ToSql,
                &f.doc as &dyn ToSql,
                //&f.mode.to_string() as &dyn ToSql,
                &f.var_arg as &dyn ToSql,
                &(f.ret.to_string()) as &dyn ToSql
            ]).map_err(|e| format!("{}", e))?;
            let func_id = self.conn.last_insert_rowid();
            Self::insert_types(&self.conn, func_id, "arg", &f.args[..])?;
            // Self::insert_types(&self.conn, func_id, "ret", &f.ret[..])?;
        }
        Ok(())
    }

    fn search_func_id(&mut self, err_qual : &str, func_name : &str) -> Result<i64, String> {
        let mut stmt = self.conn.prepare("select id from function where name = $1;").unwrap();
        let id = stmt.query_row(&[&func_name as &dyn ToSql], |row| {
            let id : i64 = row.get(0)?;
            Ok(id)
        }).map_err(|e| format!("Error retrieving {}: {}", err_qual, e) )?;
        Ok(id)
    }

    fn insert_aggregates(&mut self, id : i64, aggs : &[Aggregate]) -> Result<(), String> {
        for agg in aggs.iter() {
            let id_init = self.search_func_id(
                &format!("Init function of {} ({})", agg.name, agg.init_func),
                &agg.init_func
            )?;
            let id_state = self.search_func_id(
                &format!("State function of {} ({})", agg.name, agg.state_func),
                &agg.state_func
            )?;
            let id_final = self.search_func_id(
                &format!("Final function of {} ({})", agg.name, agg.final_func),
                &agg.final_func
            )?;
            let mut stmt_agg = self.conn.prepare("insert into aggregate \
                (lib_id, name, init, state, final) \
                values (?1, ?2, ?3, ?4, ?5);"
            ).unwrap();
            stmt_agg.execute(&[
                &id as &dyn ToSql,
                &agg.name as &dyn ToSql,
                &id_init as &dyn ToSql,
                &id_state as &dyn ToSql,
                &id_final as &dyn ToSql
            ]).map_err(|e| format!("{}", e))?;
        }
        Ok(())
    }

    fn insert_jobs(&mut self, id : i64, jobs : &[Job]) -> Result<(), String> {
        let mut stmt_job = self.conn.prepare("insert into job \
            (name, doc) values (?1, ?2);").unwrap();
        for job in jobs.iter() {
            stmt_job.execute(&[
                &job.name as &dyn ToSql,
                &job.doc as &dyn ToSql,
            ]).map_err(|e| format!("{}", e))?;
        }
        Ok(())
    }

    /* Compilation flag template */
    /*
    rustc mysrc.rs -o mysrc.so --crate-type='cdylib' \
        -L /home/diego/Software/queries/target/debug/deps \
        --extern bayes=/home/diego/Software/queries/target/debug/deps/libbayes-972493eb78ec6b73.rlib
    Each extern can be parsed from the source file 'extern crate'.*/
    fn add_source(&mut self, path_str : &str) -> Result<(String, usize), String> {
        let mut content = String::new();
        let mut f = File::open(path_str)
            .map_err(|e| format!("Could not read .rs file: {}", e) )?;
        f.read_to_string(&mut content);
        let funcs = parser::parse_top_level_funcs(&content)?;
        println!("Loaded functions = {:?}", funcs);
        Ok((String::new(), funcs.len()))
    }

    fn insert_current_lib(
        &mut self,
        lib_name : &str,
        src_path : &str,
        lib_path : &str
    ) -> Result<i64, String> {
        let mut stmt = self.conn
            .prepare("insert into library (name, srcpath, libpath, active) \
                values (?1, ?2, ?3, 1);"
            ).map_err(|e| format!("{}", e) )?;
        stmt.execute(&[lib_name.clone(), src_path.clone(), lib_path.clone()])
            .map_err(|e| format!("{}", e) )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Returns the name and number of recovered functions if successful, or an error
    /// message if unsucessful. This is used every time the user clicks the
    /// "Add library" button, and maps to an insertion into the registry
    /// database.
    pub fn add_crate(&mut self, path_str : &str) -> Result<(String, usize), String> {
        let path = Path::new(path_str);
        let mut opt_toml_path = None;
        if path.is_dir() {
            if let Ok(dir_content) = path.read_dir() {
                for entry in dir_content {
                    if let Ok(entry_path) = entry.map(|entry| entry.path()) {
                        if entry_path.extension().and_then(|e| e.to_str()) == Some("toml") {
                            opt_toml_path = Some(entry_path.to_path_buf());
                        }
                    } else {
                        return Err(String::from("Error reading entry path"));
                    }
                }
            } else {
                return Err(String::from("Could not read directory"));
            }
        } else {
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                opt_toml_path = Some(path.to_path_buf());
            } else {
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    // return self.add_source(path_str);
                    unimplemented!()
                }
                return Err(String::from("Should inform .toml or .rs file"));
            }
        };
        let toml_path = if let Some(toml_path) = opt_toml_path {
            toml_path
        } else {
            return Err(String::from(".toml manifest not found"));
        };
        let (lib_name, src_path, lib_path, funcs, aggs, jobs) = Self::parse_toml(&toml_path)?;
        self.remove_crate(&lib_name)?;
        let id = self.insert_current_lib(&lib_name, &src_path, &lib_path)?;
        self.insert_functions(id, &funcs[..])?;
        self.insert_aggregates(id, &aggs[..])?;
        self.insert_jobs(id, &jobs[..])?;
        let n = funcs.len();
        let lib = Library::new(lib_path).map_err(|e| format!("Library not found: {}", e) )?;
        self.libs.push(FunctionLibrary {
            funcs,
            aggs,
            local_path : path_str.to_string(),
            remote_path : None,
            name : lib_name.to_string(),
            lib,
            jobs,
            active : true
        });
        Ok((lib_name, n))
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
        if let Err(e) = conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true) {
            println!("{}", e);
        }
        if let Err(e) = conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, true) {
            println!("{}", e);
        }
        let libs = Vec::new();
        let mut reg = Self{ conn, libs };
        reg.reload_libs()?;
        Ok(reg)
    }

    pub fn reload_libs(&mut self) -> Result<(), &'static str> {
        let new_libs : Vec<FunctionLibrary> = Self::read_libs(&self.conn)?;
        println!("New libs: {:?}", new_libs);
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
                match Library::new(path.clone()) {
                    Ok(library) => {
                        // let funcs = FunctionLibrary::load_functions(&conn, &name[..], &libr)?;
                        let mut lib = FunctionLibrary {
                            funcs : Vec::new(),
                            aggs : Vec::new(),
                            jobs : Vec::new(),
                            local_path : path,
                            remote_path : None,
                            name : name,
                            lib : library,
                            active
                        };
                        if let Err(e) = lib.reload_functions(&conn) {
                            println!("{}", e);
                        }
                        if let Err(e) = lib.reload_aggregates(&conn) {
                            println!("{}", e);
                        }
                        if let Err(e) = lib.reload_jobs(&conn) {
                            println!("{}", e);
                        }
                        libs.push(lib);
                    },
                    Err(e) => { println!("{}", e); }
                }
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

    pub fn load_functions<'a>(&'a self) -> Result<Vec<(&'a Function, LoadedFunc<'a>)>, FunctionErr> {
        let mut active = Vec::new();
        for lib in self.libs.iter().filter(|lib| lib.active ) {
            for func in lib.funcs.iter() {
                unsafe{
                    match func.ret {
                        SqlType::I32 => {
                            let f : SqlSymbol<'a, i32> = lib.lib.get(func.name.as_bytes())
                                .map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
                            active.push((func, LoadedFunc::I32(f)));
                        },
                        SqlType::F64 => {
                            let f : SqlSymbol<'a, f64> = lib.lib.get(func.name.as_bytes())
                                .map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
                            active.push((func, LoadedFunc::F64(f)));
                        },
                        SqlType::String => {
                            let f : SqlSymbol<'a, String> = lib.lib.get(func.name.as_bytes())
                                .map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
                            active.push((func, LoadedFunc::Text(f)));
                        },
                        SqlType::Bytes => {
                            let f : SqlSymbol<'a, Vec<u8>> = lib.lib.get(func.name.as_bytes())
                                .map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
                            active.push((func, LoadedFunc::Bytes(f)));
                        },
                        _ => unimplemented!()
                    }
                }
            }
        }
        Ok(active)
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

    /*fn call<'a>(f : &'a Function, lib : &'a Library, cols : Vec<Column>) -> Result<Vec<Column>, FunctionErr> {
        unsafe {
            let f : Symbol<'a, unsafe extern fn(Vec<Column>)->Result<Vec<Column>,String>> =
                lib.get(f.name.as_bytes()).map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
            let ans = f(cols).map_err(|e| FunctionErr::UserErr(format!("{}",e)) )?;
            Ok(ans)
        }
    }*/

    pub fn try_exec_fn(
        &self,
        fn_name : String,
        arg_names : Vec<String>,
        tbl : Table
    ) -> Result<Table, FunctionErr> {
        /*if let Some((lib, f)) = self.find_fn(&fn_name[..]) {
            let cols = tbl.take_columns();
            let ans_cols = Self::call(&f, &lib.lib, cols)?;
            let mut col_names = f.get_col_names(arg_names)
                .map_err(|err_ix| FunctionErr::TableAgg(format!("Error retrieving name for column {}", err_ix)))?;
            let tbl = Table::new(col_names, ans_cols)
                .map_err(|e| FunctionErr::TableAgg(format!("{}", e)))?;
            Ok(tbl)
        } else {
            Err(FunctionErr::NotFound(tbl))
        }*/
        unimplemented!()
    }

    pub fn get_func<'a>(&'a self, name : &str) -> Option<&'a Function> {
        for lib in self.libs.iter() {
            println!("Found lib: {:?}", lib);
            if let Some(func) = lib.funcs.iter().find(|func| func.name == name) {
                println!("Found function: {:?}", func);
                return Some(func)
            }
        }
        return None
    }

    pub fn has_func_name(&self, name : &str) -> bool {
        self.get_func(name).is_some()
    }

    pub fn get_doc(&self, name : &str) -> Option<String> {
        self.get_func(name).and_then(|func| func.doc.clone() )
    }

    pub fn get_args(&self, name : &str) -> Option<Vec<String>> {
        self.get_func(name).map(|func| {
            func.args.iter().map(|a| format!("{}", a)).collect::<Vec<_>>()
        })
    }

    pub fn get_ret(&self, name : &str) -> Option<String> {
        self.get_func(name).map(|func| func.ret.to_string() )
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
    aggs : Vec<Aggregate>,
    jobs : Vec<Job>,
    local_path : String,
    remote_path : Option<String>,
    lib : Library
}

impl FunctionLibrary {

    pub fn function_names(&self) -> Vec<&str> {
        self.funcs.iter().map(|f| &f.name[..] ).collect()
    }

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

    pub fn get_fn_name(conn : &Connection, id : i64) -> Option<String> {
        let mut stmt = conn.prepare("select name from function where id = ?1").unwrap();
        stmt.query_row(&[&id as &dyn ToSql], |row| {
            let name : String = row.get(0)?;
            Ok(name)
        }).ok()
    }

    pub fn reload_aggregates(&mut self, conn : &Connection) -> Result<(), &'static str> {
        println!("Reloading aggregates");
        let mut agg_stmt = conn.prepare(
            "select aggregate.name, init, state, final \
            from aggregate inner join library \
            on library.id = aggregate.lib_id \
            where library.name = ?1;"
        ).unwrap();
        let agg_result = agg_stmt.query_map(&[&self.name], |row| {
            let init_id : i64 = row.get(1)?;
            let state_id: i64  = row.get(2)?;
            let final_id : i64 = row.get(3)?;
            let init_func = Self::get_fn_name(&conn, init_id).unwrap();
            let state_func = Self::get_fn_name(&conn, state_id).unwrap();
            let final_func = Self::get_fn_name(&conn, final_id).unwrap();
            let agg = Aggregate {
                name : row.get(0)?,
                init_func,
                state_func,
                final_func
            };
            Ok(agg)
        });
        if let Ok(res_agg) = agg_result {
            self.aggs.clear();
            for agg in res_agg {
                if let Ok(agg) = agg {
                    let init_found = self.funcs.iter()
                        .find(|f| &f.name[..] == &agg.init_func[..])
                        .is_some();
                    let state_found = self.funcs.iter()
                        .find(|f| &f.name[..] == &agg.state_func[..])
                        .is_some();
                    let final_found = self.funcs.iter()
                        .find(|f| &f.name[..] == &agg.final_func[..])
                        .is_some();
                    if init_found && state_found && final_found {
                        self.aggs.push(agg);
                    } else {
                        return Err("Missing component of aggregate function");
                    }
                } else {
                    return Err("Invalid aggregate found");
                }
            }
        } else {
            return Err("Failed at getting aggregate results");
        }
        Ok(())
    }

    pub fn reload_functions(&mut self, conn : &Connection) -> Result<(), &'static str> {
        println!("Reloading functions");
        let mut fn_stmt = conn.prepare(
            "select function.id, function.name, doc, \
            var_arg, ret from function inner join library \
            on library.id = function.lib_id \
            where library.name = ?1;"
        ).unwrap();
        let fn_result = fn_stmt.query_map(&[&self.name], |row| {
            let id = row.get::<usize, i64>(0)?;
            let name = row.get::<usize, String>(1)?;
            let doc = row.get::<usize, Option<String>>(2)?;
            // let mode = row.get::<usize, String>(3)?;
            let var_arg = row.get::<usize, bool>(3)?;
            let ret = row.get::<usize, String>(4)?;
            Ok((id, name, doc, var_arg, ret))
        });
        if let Ok(res_libs) = fn_result {
            self.funcs.clear();
            for fn_row in res_libs {
                if let Ok(fn_data) = fn_row {
                    let func_id : i64 = fn_data.0;
                    let name : String = fn_data.1;
                    let doc : Option<String> = fn_data.2;
                    // let mode : FunctionMode = fn_data.3.try_into()
                    //    .map_err(|_| "Could not pase function mode")?;
                    let var_arg = fn_data.3;
                    let ret = fn_data.4;
                    let args = Self::get_type_info(&conn, func_id, "arg");
                    // let ret = Self::get_type_info(&conn, func_id, "ret");
                    let ret = SqlType::try_from(&ret[..])
                        .map_err(|_| "Failed at converting return type")?;
                    println!("Recovered function name: {:?}", name);
                    println!("Recovered args: {:?}", args);
                    println!("Recovered ret: {:?}", ret);
                    self.funcs.push(Function {name, args, doc, ret, /*mode,*/ var_arg, /*var_ret*/ });
                } else {
                    return Err("Error retrieving nth row");
                }
            }
            println!("Lib funcs vector: {:?}", self.funcs);
            Ok(())
        } else {
            Err("Error mapping argument query results")
        }
    }

    pub fn reload_jobs(&mut self, conn : &Connection) -> Result<(), &'static str> {
        println!("Reloading jobs");
        let mut fn_stmt = conn.prepare(
            "select job.name, job.doc \
            from job inner join library \
            on library.id = job.lib_id \
            where library.name = ?1;"
        ).unwrap();
        let job_result = fn_stmt.query_map(&[&self.name], |row| {
            let name = row.get::<usize, String>(0)?;
            let doc = row.get::<usize, Option<String>>(1)?;
            Ok((name, doc))
        });
        if let Ok(jobs) = job_result {
            self.jobs.clear();
            for res_job in jobs {
                if let Ok(job) = res_job {
                    let name : String = job.0;
                    let doc : Option<String> = job.1;
                    self.jobs.push(Job{ name, doc });
                }
            }
            Ok(())
        } else {
            Err("Error mapping argument query results")
        }
    }

}

#[test]
fn load_toml() {
    let toml_path = Path::new("/home/diego/Downloads/mylib/Cargo.toml");
    println!("{:?}", FunctionLoader::parse_toml(toml_path));
}

/*#[derive(Clone, Debug/)]
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




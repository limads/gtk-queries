use postgres::{self, Client, tls::NoTls};
use sqlparser::dialect::{PostgreSqlDialect, GenericDialect};
use sqlparser::ast::{Statement, Function, Select, Value, Expr, SetExpr, SelectItem, Ident};
use sqlparser::parser::{Parser, ParserError};
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use rusqlite;
use std::fmt::Display;
use std::fmt;
use std::error::Error;
use super::table::*;
use std::path::PathBuf;
use super::postgre;
use super::sqlite;
use crate::functions::{*, function::*, loader::*};
use rusqlite::functions::*;
use libloading::Symbol;

#[cfg(feature="arrowext")]
use datafusion::execution::context::ExecutionContext;

#[cfg(feature="arrowext")]
use datafusion::datasource::csv::{CsvFile, CsvReadOptions};

// Carries a result (arranged over columns)
#[derive(Debug)]
pub enum QueryResult {
    Valid(String, Table),
    Statement(String),
    Invalid(String)
}

// TODO check UTF-8 encoding. Getting error:
// thread 'main' panicked at 'byte index 64 is not a char boundary; it is inside 'ç' (bytes 63..65) of
// When using ç in a text field.

/*pub fn as_text(r : &Row, ix : usize) -> String {
    if let Ok(str_val) = r.try_get::<usize, Option<String>>(ix) {
        match str_val {
            Some(s) => s,
            None => String::from("NULL"),
        }
    } else {
        String::from("(Unable to parse)")
    }
}

pub fn as_binary(r : &Row, ix : usize) -> String {
    if let Ok(bin_val) = r.try_get::<usize, Option<String>>(ix) {
        match bin_val {
            Some(_) => String::from("(Binary)"),
            None => String::from("NULL"),
        }
    } else {
        String::from("(Unable to parse)")
    }
}

pub fn as_bool(r : &Row, ix : usize) -> String {
    if let Ok(bool_val) = r.try_get::<usize, Option<bool>>(ix) {
        match bool_val {
            Some(b) => {
                if b {
                    String::from("True")
                } else {
                    String::from("False")
                }
            }
            None => String::from("NULL"),
        }
    } else {
        String::from("(Unable to parse)")
    }
}*/

/*pub fn as_to_string<'a, T>(r : &'a Row, ix : usize) -> String
    where T : ToString + postgres::types::FromSql<'a>
{
    if let Ok(val) = r.try_get::<usize, Option<T>>(ix) {
        match val {
            Some(v) => v.to_string(),
            None => String::from("NULL")
        }
    } else {
        String::from("(Unable to parse)")
    }
}*/

// PostgreSQL utility to pack several row results.
/*pub fn pack_postgresql_results(rows : &Vec<&Row>) -> Vec<(String, Vec<String>)> {
    let mut content = Vec::new();
    if let Some(row1) = rows.iter().next() {
        let cols = row1.columns();
        let col_types : Vec<_> = cols.iter().map(|c| c.type_()).collect();
        //let mut data : Vec<Vec<String>> = Vec::new();
        for c in cols {
            content.push( (c.name().to_string(), Vec::new()) );
        }
        // let types = cols.iter().map(|c| { c.type_})
        for i in 0..cols.len() {
            let is_bool = col_types[i] == &Type::BOOL;
            let is_bytea = col_types[i] == &Type::BYTEA;
            let is_text = col_types[i] == &Type::TEXT || col_types[i] == &Type::VARCHAR;
            let is_double = col_types[i] == &Type::FLOAT8;
            let is_float = col_types[i] == &Type::FLOAT4;
            let is_int = col_types[i] == &Type::INT4;
            let is_long = col_types[i] == &Type::INT8;
            let is_smallint = col_types[i] == &Type::INT2;
            let is_timestamp = col_types[i] == &Type::TIMESTAMP;
            let is_date = col_types[i] == &Type::DATE;
            let is_time = col_types[i] == &Type::TIME;
            for r in rows.iter() {
                if is_bool {
                    content[i].1.push(as_bool(&r, i));
                } else {
                    if is_bytea {
                        content[i].1.push(as_binary(&r, i));
                    } else {
                        if is_text {
                            content[i].1.push(as_text(&r, i));
                        } else {
                            if is_double {
                                content[i].1.push(as_to_string::<f64>(&r, i));
                            } else {
                                if is_float {
                                    content[i].1.push(as_to_string::<f32>(&r, i));
                                } else {
                                    if is_int {
                                        content[i].1.push(as_to_string::<i32>(&r, i));
                                    } else {
                                        if is_smallint {
                                            content[i].1.push(as_to_string::<i16>(&r, i));
                                        } else {
                                            if is_long {
                                                content[i].1.push(as_to_string::<i64>(&r, i));
                                            } else {
                                                if is_timestamp {
                                                content[i].1.push(as_to_string::<chrono::NaiveDateTime>(&r, i));
                                                } else {
                                                    if is_date {
                                                        content[i].1.push(as_to_string::<chrono::NaiveDate>(&r, i));
                                                    } else {
                                                        if is_time {
                                                            content[i].1.push(as_to_string::<chrono::NaiveTime>(&r, i));
                                                        } else {
                                                            content[i].1.push(String::from("(Unable to parse)"));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        println!("No rows to show");
    }
    content
}*/

/*pub fn pack_sqlite3_results(
    rows : &mut rusqlite::Rows
) ->Vec<(String, Vec<String>)> {
    let mut content = Vec::new();
    let names = rows.column_names().unwrap_or(Vec::new());
    let names : Vec<String> = names.iter()
        .map(|c| c.to_string()).collect();
    for n in &names {
        content.push( (n.clone(), Vec::new()) );
    }
    while let Ok(row) = rows.next() {
        match row {
            Some(r) => {
                for (i, n) in names.iter().enumerate() {
                    let value = r.get::<&str, rusqlite::types::Value>(&n[..])
                        .unwrap_or(rusqlite::types::Value::Null);
                    let v_text = match value {
                        rusqlite::types::Value::Null => String::from("Null"),
                        rusqlite::types::Value::Integer(int_v) => int_v.to_string(),
                        rusqlite::types::Value::Real(float_v) => float_v.to_string(),
                        rusqlite::types::Value::Text(txt_v) => txt_v,
                        rusqlite::types::Value::Blob(_) => String::from("(Binary)")
                    };
                    content[i].1.push(v_text);
                }
            },
            None => { break; }
        }
    }
    content
}*/

/*pub fn as_rows(&self) -> Vec<Vec<String>> {
    let mut rows : Vec<Vec<String>> = Vec::new();
    let keys : Vec<&String> = self.results.keys().collect();
    if self.err_msg.is_none() && keys.len() >= 1 {
        rows.push(keys.iter().map(|s| { s.to_string() }).collect());
        if let Some(col1) = self.results.get(&rows[0][0]) {
            let sz = col1.len();
            for i in 0..sz {
                let mut new_row = Vec::new();
                for k in self.results.keys() {

                    // TODO panic here
                    new_row.push(self.results[k][i].clone());
                }
                rows.push(new_row);
            }
        }
    }
    rows
}*/

    /*pub fn run(&mut self, conn : &Connection) {
        match conn.query(&self.query, &[]) {
            Ok(rows) => {
                let vec_rows : Vec<Row> = rows.iter().collect();
                self.results = self.pack_results(&vec_rows);
                self.err_msg = None;
            }
            Err(e) => {
                self.err_msg = Some(e.to_string());
            }
        }
    }*/

    /* Given a vector of column names, return vector of numeric
    results, taken from a vector of queries */
    /*pub fn split_cols_from_queries<'a>(
        queries : &Vec<PostgreQuery>,
        cols : Vec<&str>)
    -> Result<Vec<Vec<f64>>,&'a str> {
            let mut res = Vec::new();

            for c in cols {
                for q in queries {
                    let maybe_found = q.results.keys().find(|k| {
                        *k == c
                    });
                    match maybe_found {
                        Some(k) => {
                            if let Ok(mut r) = q.cols_as_numbers(vec![k]) {
                                if let Some(c) = r.pop() {
                                    res.push(c);
                                }
                                break;
                            } else {
                                return Err("Impossible to parse column");
                            }
                         },
                        None => { return Err("Key not found"); }
                    };
                };
            }
            Ok(res)
    }*/

    /*pub fn agg_queries_as_numbers(
        queries : &Vec<PostgreQuery>,
        cols : Vec<&str>) -> Result<Vec<Vec<f64>>,&str> {

    }*/
//}

/*fn get_form_item<W>(bx : gtk::Box, r : usize, c : usize)
    -> W where W : IsA<gtk::Widget> {
    let row : gtk::Box = bx.get_children()[r]
        .downcast().unwrap();
    let w : W = row.get_children()[c].downcast().unwrap();
    w
}*/

#[derive(Debug, Clone)]
pub struct Substitution {
    proj_ix : usize,
    func_name : String,
    func_args : Vec<String>
}

fn split_function(f : Function) -> Substitution {
    let mut args = Vec::new();
    for a in f.args {
        match a {
            Expr::Identifier(id) => args.push(id.value),
            Expr::Wildcard => args.push(String::from("*")),
            Expr::Value(v) => match v {
                Value::Number(n) => args.push(n),
                Value::SingleQuotedString(s) => args.push(s),
                Value::Boolean(b) => args.push(b.to_string()),
                Value::Null => args.push(String::from("NULL")),
                _ => { }
            },
            Expr::QualifiedWildcard(ws) => {
                for w in ws {
                    args.push(w.to_string())
                }
            },
            Expr::CompoundIdentifier(ids) => {
                for id in ids {
                    args.push(id.to_string())
                }
            },
            _ => { }
        }
    }
    Substitution{ proj_ix : 0, func_name : f.name.to_string(), func_args : args }
}

/// If query has a single function call statement, separate it for client-side
/// execution while the naked arguments are sent to the database. Pass the statement
/// unchanged and None otherwise.
fn filter_single_function_out(stmt : &Statement) -> (Statement, Option<Substitution>) {
    let mut transf_stmt = stmt.clone();
    let sub : Option<Substitution> = match transf_stmt {
        Statement::Query(ref mut q) => match q.body {
            SetExpr::Select(ref mut sel) => {
                if sel.projection.len() == 1 {
                    if let Some(proj) = sel.projection.iter().next().cloned() {
                        match proj {
                            SelectItem::ExprWithAlias{ expr, .. } | SelectItem::UnnamedExpr(expr) => {
                                match expr {
                                    Expr::Function(func) => {
                                        let sub = split_function(func);
                                        sel.projection.remove(0);
                                        for name in sub.func_args.iter().rev() {
                                            sel.projection.push(SelectItem::UnnamedExpr(Expr::Identifier(Ident::new(name))));
                                        }
                                        Some(sub)
                                    },
                                    _ => None
                                }
                            },
                            _ => None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            _ => None,
        },
        _ => None
    };
    (transf_stmt, sub)
}

// TODO SQL parser is not accepting PostgreSQL double precision types
// Use this if client-side parsing is desired.
pub fn parse_sql(sql : &str) -> Result<Vec<Statement>, String> {
    //let dialect = PostgreSqlDialect {};
    let dialect = GenericDialect {};
    Parser::parse_sql(&dialect, &sql[..])
        .map_err(|e| {
            match e {
                ParserError::TokenizerError(s) => s,
                ParserError::ParserError(s) => s
            }
        })
}

/*/// Remove the content from all string literals from a SQL query.
fn remove_string_literals(text : &str) -> String {
    let split_text = text.split("\"|$$|'");
    let mut out = String::new();
    for (i, s) in split_text {
        if  i % 2 == 0 {
            out += &format!("{}\"\""s);
        }
    }
    out
}*/

/// Parse a SQL String, separating the queries.
/// Use this if no client-side parsing is desired.
/// TODO remove ; from text literals "" and $$ $$ when doing the
/// analysis. Returns true for each statement if the resulting statement
/// is a select.
pub fn split_sql(sql_text : String) -> Result<Vec<(String, bool)>, String> {
    let stmts_strings : Vec<_> = sql_text.split(";")
        .filter(|c| c.len() > 0 && *c != "\n" && *c != " " && *c != "\t")
        .map(|c| c.trim_start().trim_end().to_string()).collect();
    let mut stmts = Vec::new();
    // TODO this will break if literals contain select/with statements.
    for stmt in stmts_strings {
        println!("{}", stmt);
        let is_select = stmt.starts_with("select") || stmt.starts_with("SELECT") ||
            (stmt.starts_with("with") && (stmt.contains("select") || stmt.contains("SELECT"))) ||
            (stmt.starts_with("WITH") && (stmt.contains("select") || stmt.contains("SELECT")));
        stmts.push((stmt.clone(), is_select));
    }
    Ok(stmts)
}

pub fn sql2table(result : Result<Vec<Statement>, String>) -> String {
    format!("{:?}", result)
}

pub fn make_query(query : &str) -> String {
    sql2table(parse_sql(query))
}

pub enum SqlEngine {
    Inactive,
    Local{conn : rusqlite::Connection },
    PostgreSql{conn_str : String, conn : postgres::Client },
    Sqlite3{path : Option<PathBuf>, conn : rusqlite::Connection},

    #[cfg(feature="arrowext")]
    Arrow{ ctx : ExecutionContext }
}

#[derive(Debug)]
pub struct DecodingError {
    msg : &'static str
}

impl Display for DecodingError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", self.msg)
    }
}

impl Error for DecodingError {

}

impl DecodingError {

    pub fn new(msg : &'static str) -> Box<Self> {
        Box::new(DecodingError{ msg })
    }

}

impl SqlEngine {

    /*fn attach_functions(conn : &rusqlite::Connection) {
        // generate N ordered real elements from a memory-contiguous
        // byte array decodable as f64 (double precision)
        let create_scalar_ok = conn.create_scalar_function("jdecode", 1, false, move |ctx| {
            if ctx.len() != 1 {
                println!("Function receives single argument");
                return Err(rusqlite::Error::UserFunctionError(
                    DecodingError::new("Function receives single argument")
                ));
            }

            let res_buf = ctx.get::<Vec<u8>>(0);
            match res_buf {
                Ok(buf) => {
                    match decoding::decode_bytes(&buf[..]) {
                        Some(data) => {
                            if data.len() >= 1 {
                                let mut json = String::from("{");
                                //println!("{:?}", data);
                                for (i, d) in data.iter().enumerate() {
                                    json += &format!("{:.8}", d)[..];
                                    if i < data.len()-1 {
                                        json += ","
                                    } else {
                                        json += "}"
                                    }
                                    if i < 10 {
                                        println!("{}", d);
                                    }
                                }
                                Ok(json)
                            } else {
                                println!("Empty buffer");
                                Err(rusqlite::Error::UserFunctionError(
                                    DecodingError::new("Empty buffer")
                                ))
                            }
                        },
                        None => {
                            println!("Could not decode data");
                            Err(rusqlite::Error::UserFunctionError(
                                    DecodingError::new("Could not decode data")
                                ))
                        }
                    }
                },
                Err(e) => {
                    println!("{}", e);
                    Err(rusqlite::Error::UserFunctionError(
                        DecodingError::new("Field is not a blob")
                    ))
                }
            }
        });

        let my_fn = move |_ : Table| { String::from("Hello") };
        let agg = TableAggregate::<String>{
            ans : String::new(),
            f : &my_fn
        };
        let create_agg_ok = conn.create_aggregate_function("multi",
            2,
            false,
            agg
        );
        match create_agg_ok {
            Ok(_) => { },
            Err(e) => { println!("{}", e); }
        }
        match create_scalar_ok {
            Ok(_) => { },
            Err(e) => { println!("{}", e); }
        }
    }

    /*fn load_extension(
        conn : &rusqlite::Connection,
        path : &str
    ) {
        match conn.load_extension(path, None) {
            Ok(_) => { },
            Err(e) => { println!("{}", e); }
        }
    }*/

    /// Given a vector of paths to be loaded,
    fn load_extensions(
        conn : &rusqlite::Connection,
        paths : Vec<String>
    ) {
        for p in paths.iter() {
            Self::load_extension(conn, &p[..]);
        }
    }*/

    pub fn try_new_postgre(conn_str : String) -> Result<Self,String> {
        let tls_mode = NoTls{ };
        //println!("{}", conn_str);
        match Client::connect(&conn_str[..], tls_mode) {
            Ok(conn) => Ok(SqlEngine::PostgreSql{ conn_str, conn }),
            Err(e) => Err(e.to_string())
        }
    }

    fn bind_sqlite3_udfs(conn : &rusqlite::Connection, loader : &FunctionLoader) {
        println!("Function loader state (New Sqlite3 conn): {:?}", loader);
        match loader.load_functions() {
            Ok(funcs) => {
                for (func, load_func) in funcs {
                    let n_arg = if func.var_arg {
                        -1
                    } else {
                        func.args.len() as i32
                    };
                    let created = match load_func {
                        LoadedFunc::F64(f) => {
                            // Since we are handing over control of the function to the C
                            // SQLite API, we can't track the lifetime anymore. raw_fn is now
                            // assumed to stay alive while the last shared reference to the
                            // function loader is alive and the library has not been cleared
                            // from the "libs" array of loader. Two things mut happen to guarantee this:
                            // (1) The function is always removed when the library is removed, so this branch is
                            // not accessed;
                            // (2) The function is removed from the Sqlite connection via conn.remove_function(.)
                            // any time the library is de-activated.
                            // (3) No call to raw_fn must happen outside the TableEnvironment public API,
                            // (since TableEnvironment holds an Arc copy to FunctionLoader).
                            let raw_fn = unsafe { f.into_raw() };
                            conn.create_scalar_function(
                                &func.name,
                                n_arg,
                                FunctionFlags::empty(),
                                move |ctx| { unsafe{ raw_fn(ctx) } }
                            )
                        },
                        _ => unimplemented!()
                    };
                    if let Err(e) = created {
                        println!("{:?}", e);
                    } else {
                        println!("User defined function {:?} registered", func);
                    }
                }
            },
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }

    pub fn try_new_sqlite3(path : Option<PathBuf>, loader : &Arc<Mutex<FunctionLoader>>) -> Result<Self, String> {
        let res_conn = match &path {
            Some(ref path) => rusqlite::Connection::open(path),
            None => {
                let conn = rusqlite::Connection::open_in_memory()
                    .and_then(|conn| {
                        rusqlite::vtab::csvtab::load_module(&conn)
                            .map_err(|e| format!("{}", e));
                        if let Ok(loader) = loader.lock() {
                            Self::bind_sqlite3_udfs(&conn, &*loader);
                        } else {
                            println!("Unable to acquire lock over function loader");
                        }
                        Ok(conn)
                    });
                conn
            }
        };
        match res_conn {
            Ok(conn) => {
                // Self::attach_functions(&conn);
                // let lib = libloading::Library::new("/home/diego/Software/mvlearn-sqlite/target/debug/libmvlearn.so").expect("Library not found");
                // unsafe {
                //    let func: libloading::Symbol<unsafe extern fn(rusqlite::Row)->rusqlite::Row> = lib.get(b"process_row").expect("Function not found");
                // func();
                //}
                Ok(SqlEngine::Sqlite3{path, conn})
            },
            Err(e) => Err(format!("{}", e))
        }
    }

    pub fn try_new_local(_content : String) -> Result<Self, String> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| format!("{}", e))?;
        // let guard = rusqlite::LoadExtensionGuard::new(&conn)
        //    .map_err(|e| format!("{}", e))?;
        // conn.load_extension(Path::new("csv"), None);
        Ok(SqlEngine::Local{conn})
    }

    /// Inserts a table, but only if using in-memory SQLite3 database
    pub fn insert_external_table(&mut self, tbl : &Table) {
        match &self {
            SqlEngine::Sqlite3{path, conn : _} => {
                match &path {
                    None => {
                        if let Ok(q) = tbl.sql_string("transf_table") {
                            println!("{}", q);
                            if let Err(e) = self.try_run(q, true, None) {
                                println!("{}", e);
                            }
                        } else {
                            println!("Tried to generate SQL for unnamed table");
                        }
                    },
                    Some(_) => {
                        println!("Can only insert tables to in-memory SQLite3 databases");
                    }
                }
            },
            _ => {
                println!("Tried to insert table to Non-sqlite3 database");
            }
        }
    }

    /*pub fn get_table_names(&mut self) -> Option<Vec<DBObject>> {
        match &self {
            SqlEngine::Sqlite3{path : _, conn : _} => {
                let ans = self.try_run(
                    "select name from sqlite_master where type='table';".into()
                ).map_err(|e| println!("{}", e) ).ok()?;
                let q_res = ans.get(0)?;
                match q_res {
                    QueryResult::Valid(names) => {
                        let names = names.get(0)?.1.clone();
                        let mut objs = Vec::new();
                        for name in names {
                            let ans = self.try_run(format!("pragma table_info({});", name))
                                .map_err(|e| println!("{}", e) ).ok()?;
                            let q_res = ans.get(0)?;
                            match q_res {
                                QueryResult::Valid(col_info) => {
                                    let names = &col_info.get(1)?.1;
                                    let col_types = &col_info.get(2)?.1;
                                    let info : Vec<(String, String)> =
                                        names.iter().zip(col_types.iter())
                                        .map(|(s1, s2)| (s1.clone(), s2.clone() ))
                                        .collect();
                                    let obj = DBObject::new_table(&name, info);
                                    objs.push(obj);
                                },
                                QueryResult::Invalid(msg) => { println!("{}", msg); return None; },
                                QueryResult::Statement(_) => { return None; }
                            }
                        }
                        Some(objs)
                    },
                    QueryResult::Invalid(msg) => {
                        println!("{}", msg);
                        None
                    },
                    QueryResult::Statement(_) => {
                        None
                    }
                }
            },
            _ => None
        }
    }*/

    /// Table is an expesive data structure, so we pass ownership to the function call
    /// because it may be disassembled if the function is found, but we return it back to
    /// the user on an not-found error, since the caller will want to re-use it.
    fn try_client_function(sub : Substitution, tbl : Table, loader : &FunctionLoader) -> QueryResult {
        match loader.try_exec_fn(sub.func_name, sub.func_args, tbl) {
            Ok(tbl) => QueryResult::Valid(String::new(), tbl),
            Err(FunctionErr::UserErr(msg)) | Err(FunctionErr::TableAgg(msg)) => {
                QueryResult::Invalid(msg)
            },
            Err(FunctionErr::TypeMismatch(ix)) => {
                QueryResult::Invalid(format!("Type mismatch at column {}", ix))
            },
            Err(FunctionErr::NotFound(tbl)) => {
                QueryResult::Valid(String::new(), tbl)
            }
        }
    }

    fn query_postgre(conn : &mut postgres::Client, q : &str) -> QueryResult {
        match conn.query(q, &[]) {
            Ok(rows) => {
                match postgre::build_table_from_postgre(&rows[..]) {
                    Ok(tbl) => {
                        QueryResult::Valid(q.to_string(), tbl)
                    },
                    Err(e) => QueryResult::Invalid(e.to_string())
                }
            },
            Err(e) => {
                QueryResult::Invalid(e.to_string())
            }
        }
    }

    fn query_sqlite(conn : &mut rusqlite::Connection, q : &str) -> QueryResult {
        match conn.prepare(q) {
            Ok(mut prep_stmt) => {
                match prep_stmt.query(rusqlite::NO_PARAMS) {
                    Ok(rows) => {
                        match sqlite::build_table_from_sqlite(rows) {
                            Ok(tbl) => {
                                QueryResult::Valid(q.to_string(), tbl)
                            },
                            Err(e) => {
                                println!("Error building table: {}", e);
                                QueryResult::Invalid(e.to_string())
                            }
                        }
                    },
                    Err(e) => {
                        QueryResult::Invalid(e.to_string())
                    }
                }
            },
            Err(e) => {
                QueryResult::Invalid(e.to_string())
            }
        }
    }

    fn exec_postgre(conn : &mut postgres::Client, e : &str) -> QueryResult {
        match conn.execute(e, &[]) {
            Ok(n) => QueryResult::Statement(format!("{} row(s) modified", n)),
            Err(e) => QueryResult::Invalid(e.to_string())
        }
    }

    fn exec_sqlite(conn : &mut rusqlite::Connection, e : &str) -> QueryResult {
        match conn.execute(e, rusqlite::NO_PARAMS) {
            Ok(n) => QueryResult::Statement(format!("{} row(s) modified", n)),
            Err(e) => QueryResult::Invalid(e.to_string())
        }
    }

    #[cfg(feature="arrowext")]
    fn query_arrow(ctx : &mut ExecutionContext, q : &str) -> QueryResult {
        match ctx.sql(q, 10000) {
            Ok(results) => {
                if results.len() == 0 {
                    return QueryResult::Statement(String::from("0 Row(s) modified"));
                } else {
                    match super::arrow::table_from_batch(&results[0]) {
                        Ok(tbl) => QueryResult::Valid(q.to_string(), tbl),
                        Err(e) => QueryResult::Invalid(format!("{}", e))
                    }
                }
            },
            Err(e) => {
                QueryResult::Invalid(format!("{}", e) )
            }
        }
    }

    #[cfg(feature="arrowext")]
    fn exec_arrow(ctx : &mut ExecutionContext, q : &str) -> QueryResult {
        match ctx.sql(q, 10000) {
            Ok(results) => {
                if results.len() == 0 {
                    return QueryResult::Statement(String::from("0 Row(s) modified"));
                } else {
                    match super::arrow::table_from_batch(&results[0]) {
                        Ok(tbl) => QueryResult::Valid(q.to_string(), tbl),
                        Err(e) => QueryResult::Invalid(format!("{}", e))
                    }
                }
            },
            Err(e) => {
                QueryResult::Invalid(format!("{}", e) )
            }
        }
    }

    /// Runs the informed query sequence without client-side parsing.
    pub fn run_any(&mut self, query_seq : String) -> Result<Vec<QueryResult>, String> {
        let stmts = split_sql(query_seq).map_err(|e| format!("{}", e) )?;
        let mut results = Vec::new();
        // TODO disregard select and with from literals.
        for (stmt, is_select) in stmts {
            match self {
                SqlEngine::Inactive => { return Err(String::from("Inactive Sql engine")); },
                SqlEngine::PostgreSql{ conn_str : _ , conn } => {
                    if is_select {
                        results.push(Self::query_postgre(conn, &format!("{}", stmt)));
                    } else {
                        results.push(Self::exec_postgre(conn, &format!("{}", stmt)));
                    }
                },
                SqlEngine::Sqlite3{ path : _, conn} | SqlEngine::Local{ conn } => {
                    if is_select {
                        results.push(Self::query_sqlite(conn, &format!("{}", stmt)));
                    } else {
                        results.push(Self::exec_sqlite(conn, &format!("{}", stmt)));
                    }
                },

                #[cfg(feature="arrowext")]
                SqlEngine::Arrow{ ctx } => {
                    if is_select {
                        results.push(Self::query_arrow(ctx, &stmt));
                    } else {
                        results.push(Self::exec_arrow(ctx, &stmt));
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn try_run(
        &mut self,
        query_seq : String,
        parse : bool,
        loader : Option<&FunctionLoader>
    ) -> Result<Vec<QueryResult>, String> {
        let stmts = match parse {
            true => match parse_sql(&query_seq) {
                Ok(stmts) => stmts,
                Err(_) => return self.run_any(query_seq)
            }
            false => return self.run_any(query_seq)
        };
        let mut results = Vec::new();
        if stmts.len() == 0 {
            return Err(String::from("Empty query sequence"));
        }
        match self {
            SqlEngine::Inactive => { return Err(String::from("Inactive Sql engine")); },
            SqlEngine::PostgreSql{ conn_str : _ , conn } => {
                for stmt in stmts {
                    // let (stmt, opt_sub) = filter_single_function_out(&stmt);
                    let stmt_string = stmt.to_string();
                    match stmt {
                        Statement::Query(q) => {
                            results.push(Self::query_postgre(conn, &format!("{}", q)));
                        },
                        stmt => {
                            results.push(Self::exec_postgre(conn, &format!("{}", stmt)));
                        }
                    }
                }
            },
            SqlEngine::Sqlite3{ path : _, conn} | SqlEngine::Local{ conn } => {
                // conn.execute() for insert/update/delete
                for stmt in stmts {
                    //let (stmt, opt_sub) = filter_single_function_out(&stmt);
                    let stmt_string = stmt.to_string();
                    match stmt {
                        Statement::Query(q) => {
                            // println!("Sending query: {}", q);
                            results.push(Self::query_sqlite(conn, &format!("{}", q)));
                        },
                        stmt => {
                            results.push(Self::exec_sqlite(conn, &format!("{}", stmt)));
                        }
                    }
                }
            },

            #[cfg(feature="arrowext")]
            SqlEngine::Arrow{ ctx } => {
                for stmt in stmts {
                    match stmt {
                        Statement::Query(q) => {
                            results.push(Self::query_arrow(ctx, &format!("{}", q)));
                        },
                        stmt => {
                            results.push(Self::exec_arrow(ctx, &format!("{}", stmt)));
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn backup_if_sqlite(&self, path : PathBuf) {
        match self {
            SqlEngine::Sqlite3{ path: _, conn } => {
                if let Err(e) = conn.backup(rusqlite::DatabaseName::Main, path, None) {
                    println!("{}", e);
                }
            },
            _ => {
                println!("Connection is not an SQLite one");
            }
        }
    }

}

/*pub fn try_run_all(&mut self) {
    if let Ok(mut maybe_conn) = self.rc_conn.clone().try_borrow_mut() {
        //let maybe_conn = *maybe_conn;
        if let Some(mut c) = maybe_conn.as_mut() {
            for q in self.queries.iter_mut() {
                q.run(&c);
                if let Some(msg) = &q.err_msg {
                    println!("{}", msg);
                }
            }
        }
    }
}

pub fn try_run_some(&mut self) {
    if let Ok(mut maybe_conn) = self.rc_conn.clone().try_borrow_mut() {
        if let Some(mut c) = maybe_conn.as_mut() {
            println!("valid queries : {:?}", self.valid_queries);
            for i in self.valid_queries.iter() {
                if let Some(mut q) = self.queries.get_mut(*i) {
                    q.run(&c);
                    if let Some(msg) = &q.err_msg {
                        println!("{}", msg);
                    }
                }
            }
        } else {
            println!("No connections available");
        }
    }
}*/

/*pub fn mark_all_valid(&mut self) {
    self.valid_queries = (0..self.queries.len()).collect();
}*/
/*pub fn get_valid_queries(&self) -> Vec<&PostgreQuery> {
    let mut queries : Vec<&PostgreQuery> = Vec::new();
    //for q in self.queries.iter() {
    //if q.err_msg.is_none() {
    //    valid_queries.push(&q);
    //}
    //}
    for i in self.valid_queries.iter() {
        if let Some(q) = self.queries.get(*i) {
            queries.push(q);
        }
    }
    queries
}*/

/*pub fn get_valid_queries_code(&self) -> Vec<String> {
    let queries = self.get_valid_queries();
    queries.iter().map(|q|{ q.query.clone() }).collect()
}

pub fn get_all_queries_code(&self) -> Vec<&str> {
    self.queries.iter().map(|q| { q.query.as_str() }).collect()
}

pub fn get_subset_valid_queries(
    &self,
    idx : Vec<usize>)
-> Vec<&PostgreQuery> {
    let queries = self.get_valid_queries().clone();
    let mut keep_queries = Vec::new();
    for i in idx {
        keep_queries.push(queries[i]);
    }
    keep_queries
}*/

pub struct SqlListener {
    _handle : JoinHandle<()>,
    ans_receiver : Receiver<Vec<QueryResult>>,

    /// Carries a query sequence and whether this query should be parsed at the client
    cmd_sender : Sender<(String, bool)>,
    pub engine : Arc<Mutex<SqlEngine>>,
    pub last_cmd : Arc<Mutex<Vec<String>>>,
    //loader : Arc<Mutex<FunctionLoader>>
}

impl SqlListener {

    pub fn launch(loader : Arc<Mutex<FunctionLoader>>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<(String, bool)>();
        let (ans_tx, ans_rx) = mpsc::channel::<Vec<QueryResult>>();

        let engine = Arc::new(Mutex::new(SqlEngine::Inactive));
        let engine_c = engine.clone();

        // Must join on structure desctruction.
        let r_thread = thread::spawn(move ||  {
            let loader = loader.clone();
            loop {
                // TODO perhaps move SQL parsing to here so loader is passed to
                // try_run iff there are local functions matching the query.
                match (cmd_rx.recv(), engine_c.lock(), loader.lock()) {
                    (Ok((cmd, parse)), Ok(mut eng), Ok(loader)) => {
                        let result = eng.try_run(cmd, parse, Some(&loader));
                        match result {
                            Ok(ans) => {
                                if let Err(e) = ans_tx.send(ans) {
                                    println!("{}", e);
                                }
                            },
                            Err(e) => {
                                let inv_res = vec![QueryResult::Invalid( e.to_string() )];
                                if let Err(e) = ans_tx.send(inv_res) {
                                    println!("{}", e);
                                }
                            }
                        }
                    },
                    _ => {
                        panic!("Failed to acquire lock over engine");
                    }

                }
            }
        });
        Self {
            _handle : r_thread,
            ans_receiver : ans_rx,
            cmd_sender : cmd_tx,
            engine : engine,
            last_cmd : Arc::new(Mutex::new(Vec::new()))
        }
    }

    /// Tries to parse SQL at client side. If series of statements at string
    /// are correctly parsed, send the SQL to the server. If sequence is not
    /// correctly parsed, do not send anything to the server, and return the
    /// error to the user.
    pub fn send_command(&self, sql : String, parse : bool) -> Result<(), String> {
        if let Ok(mut last_cmd) = self.last_cmd.lock() {
            last_cmd.clear();
            match parse {
                true => {
                    match parse_sql(&sql[..]) {
                        Ok(stmts) => {
                            for stmt in stmts.iter() {
                                let stmt_txt = match stmt {
                                    Statement::Query(_) => String::from("select"),
                                    _ => String::from("other")
                                };
                                last_cmd.push(stmt_txt);
                            }
                        },
                        Err(e) => {
                            for (stmt, is_select) in split_sql(sql.clone())? {
                                let stmt_txt = match is_select {
                                    true => String::from("select"),
                                    false => String::from("other")
                                };
                                last_cmd.push(stmt_txt);
                            }
                        }
                    }
                },
                false => {
                    last_cmd.push(String::from("other"));
                }
            }
        } else {
            return Err(format!("Unable to acquire lock over last commands"));
        }
        self.cmd_sender.send((sql.clone(), parse))
            .expect("Error sending SQL command over channel");
        Ok(())
    }

    /// Gets all results which might have been queued at the receiver.
    pub fn maybe_get_result(&self) -> Option<Vec<QueryResult>> {
        let mut full_ans = Vec::new();
        while let Ok(ans) = self.ans_receiver.try_recv() {
            full_ans.extend(ans);
        }
        if full_ans.len() > 0 {
            Some(full_ans)
        } else {
            None
        }
    }
}

/*fn run_expression(
    mut table : String,
    name : Option<String>,
    mut expr : String,
) -> Result<String, String> {

    /*if let Some(n) = name {
        let prefix = n + " = X; ";
        expr = prefix + &expr[..];
    }
    let mut arg_expr = String::from("-e '");
    arg_expr = arg_expr + &expr[..] + "'";
    let spawned_cmd = Command::new("r")
        .stdin(Stdio::piped());

    spawned_cmd.stdin.unwrap()
        .arg("-d")  // Evaluate stdin as CSV input
        .arg("-p")  // Output last evaluated expression
        .arg(&arg_expr[..])
        .spawn();
    println!("Command : {:?}", spawned_cmd);

    // output.status
    // output.stdout
    // output.stderr

    match spawned_cmd {
        Ok(cmd) => {
            let mut cmd_stdin = cmd.stdin.unwrap();
            println!("STDIN : {:?}", table);
            let mut writer = BufWriter::new(&mut cmd_stdin);
            if let Err(e) = writer.write_all(&mut table.as_bytes()) {
                println!("Error : {}", e);
                return Err(format!("{}", e));
            }
            match cmd.stdout {
                Some(mut out) => {
                    let mut content = Vec::new();
                    if let Ok(_) = out.read(&mut content) {
                        if let Ok(utf8) = String::from_utf8(content) {
                            Ok(utf8)
                        } else {
                            Err("Could not parse result as UTF-8".into())
                        }
                    } else {
                        Err("Could not read result into string".into())
                    }
                },
                None => Err("Could not recover stdout hande".into())
            }
        },
        Err(e) => { return Err(e.to_string()); }
    }*/
    // Err("Unimplemented".into())

    Ok(make_query(&expr[..]))
}*/




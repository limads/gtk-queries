use postgres::{self, Client, tls::NoTls};
use postgres::row::Row;
use postgres::types::Type;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::ast::Statement;
use sqlparser::parser::{Parser, ParserError};
use std::process::{Command, Stdio};
use std::io::{BufWriter, Write, BufReader, Read};
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use rusqlite;
use std::path::Path;
//use crate::decoding;
use std::fmt::Display;
use std::fmt;
use std::error::Error;
use super::table::*;
use libloading;
use rusqlite::functions::*;
use chrono::NaiveDateTime;
//use serde_json::value::Value;
use std::path::PathBuf;
use super::postgre;
use super::sqlite;

// Carries a result (arranged over columns)
#[derive(Debug)]
pub enum QueryResult {
    Valid(Table),
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

/// Parse a SQL String, separating the queries.
pub fn split_sql(sql_text : String) -> Vec<String> {
    sql_text.split(";")
        .filter(|c| c.len() > 0 && *c != "\n" && *c != " " && *c != "\t")
        .map(|c| c.to_string()).collect()
}

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

// TODO SQL parser is not accepting PostgreSQL double precision types
pub fn parse_sql(sql : &str) -> Result<Vec<Statement>, String> {
    let dialect = PostgreSqlDialect {};
    Parser::parse_sql(&dialect, sql.to_string())
        .map_err(|e| {
            match e {
                ParserError::TokenizerError(s) => s,
                ParserError::ParserError(s) => s
            }
        })
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
    Sqlite3{path : Option<PathBuf>, conn : rusqlite::Connection}
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

    pub fn try_new_sqlite3(path : Option<PathBuf>) -> Result<Self, String> {
        let res_conn = match &path {
            Some(ref path) => rusqlite::Connection::open(path),
            None => rusqlite::Connection::open_in_memory()
        };
        match res_conn {
            Ok(conn) => {
                // Self::attach_functions(&conn);
                // let lib = libloading::Library::new("/home/diego/Software/mvlearn-sqlite/target/debug/libmvlearn.so").expect("Library not found");
                // unsafe {
                //    let func: libloading::Symbol<unsafe extern fn(rusqlite::Row)->rusqlite::Row> = lib.get(b"process_row").expect("Function not found");
                //func();
                //}
                Ok(SqlEngine::Sqlite3{path, conn})
            },
            Err(e) => Err(format!("{}", e))
        }
    }

    pub fn try_new_local(content : String) -> Result<Self, String> {
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
                        if let Some(q) = tbl.sql_string("transf_table") {
                            println!("{}", q);
                            if let Err(e) = self.try_run(q) {
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

    pub fn try_run(
        &mut self,
        query_seq : String
    ) -> Result<Vec<QueryResult>, String> {
        let stmts = parse_sql(&query_seq).map_err(|e| format!("{}", e) )?;
        let mut results = Vec::new();
        if stmts.len() == 0{
            return Err(String::from("Empty query sequence"));
        }
        match self {
            SqlEngine::Inactive => { return Err(String::from("Inactive Sql engine")); },
            SqlEngine::PostgreSql{ conn_str : _ , conn : conn } => {
                for stmt in stmts {
                    match stmt {
                        Statement::Query(q) => {
                            match conn.query(&format!("{}", q)[..], &[]) {
                                Ok(rows) => {
                                    // let vec_rows : Vec<&Row> = rows.iter().collect();
                                    match postgre::build_table_from_postgre(&rows[..]) {
                                        Ok(tbl) => results.push(QueryResult::Valid(tbl)),
                                        Err(e) => results.push(QueryResult::Invalid(e.to_string()))
                                    }
                                },
                                Err(e) => {
                                    results.push(QueryResult::Invalid(e.to_string()));
                                }
                            }
                        },
                        stmt => {
                            match conn.execute(&format!("{}", stmt)[..], &[]) {
                                Ok(n) => results.push(QueryResult::Statement(format!("{} row(s) modified", n))),
                                Err(e) => results.push(QueryResult::Invalid(e.to_string()))
                            }
                        }
                    }
                }
            },
            SqlEngine::Sqlite3{ path : _, conn : conn} => {
                // conn.execute() for insert/update/delete
                for stmt in stmts {
                    match stmt {
                        Statement::Query(q) => {
                            match conn.prepare(&format!("{}",q)[..]) {
                                Ok(mut stmt) => {
                                    match stmt.query(rusqlite::NO_PARAMS) {
                                        Ok(rows) => {
                                            match sqlite::build_table_from_sqlite(rows) {
                                                Ok(tbl) => results.push(QueryResult::Valid(tbl)),
                                                Err(e) => results.push(QueryResult::Invalid(e.to_string()))
                                            }
                                        },
                                        Err(e) => {
                                            results.push(QueryResult::Invalid(e.to_string()));
                                        }
                                    }
                                },
                                Err(e) => {
                                    results.push(QueryResult::Invalid(e.to_string()));
                                }
                            }
                        },
                        stmt => {
                            match conn.execute(&format!("{}", stmt)[..], rusqlite::NO_PARAMS) {
                                Ok(n) => results.push(QueryResult::Statement(format!("{} row(s) modified", n))),
                                Err(e) => results.push(QueryResult::Invalid(e.to_string()))
                            }
                        }
                    }
                }
            },
            _ => { return Err(String::from("Current SQL Engine does not support queries.")); }
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
    handle : JoinHandle<()>,
    ans_receiver : Receiver<Vec<QueryResult>>,
    cmd_sender : Sender<String>,
    pub engine : Arc<Mutex<SqlEngine>>,
    pub last_cmd : Arc<Mutex<Vec<String>>>
}

impl SqlListener {

    pub fn launch() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>();
        let (ans_tx, ans_rx) = mpsc::channel::<Vec<QueryResult>>();

        let engine = Arc::new(Mutex::new(SqlEngine::Inactive));
        let engine_c = engine.clone();

        // Must join on structure desctruction.
        let r_thread = thread::spawn(move ||  {
            loop {
                match (cmd_rx.recv(), engine_c.lock()) {
                    (Ok(cmd), Ok(mut eng)) => {
                        let result = eng.try_run(cmd);
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
                        println!("Failed to acquire lock over engine");
                    }

                }
            }
        });
        Self {
            handle : r_thread,
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
    pub fn send_command(&self, sql : String) -> Result<(), String> {
        if let Ok(mut last_cmd) = self.last_cmd.lock() {
            last_cmd.clear();
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
                    return Err(e.to_string());
                }
            }
        } else {
            return Err(format!("Unable to acquire lock over last commands"));
        }
        self.cmd_sender.send(sql.clone())
            .expect("Error sending SQL command over channel");
        Ok(())
    }

    pub fn maybe_get_result(&self) -> Option<Vec<QueryResult>> {
        if let Ok(ans) = self.ans_receiver.try_recv() {
            // println!("{:?}", ans);
            Some(ans)
        } else {
            // println!("Unable to acquire lock");
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




use postgres::{self, Client, tls::NoTls};
use sqlparser::dialect::{PostgreSqlDialect, GenericDialect};
use sqlparser::ast::{Statement, Function, Select, Value, Expr, SetExpr, SelectItem, Ident, TableFactor, Join, JoinOperator};
use sqlparser::parser::{Parser, ParserError};
use sqlparser::dialect::keywords::Keyword;
use std::sync::mpsc::{self, Sender, Receiver};
use sqlparser::tokenizer::{Tokenizer, Token, Word, Whitespace};
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
use crate::functions::{function::*, loader::*};
use rusqlite::functions::*;
use crate::tables::environment::{DBObject, DBType};
use std::convert::TryInto;
use std::collections::HashMap;
use regex::Regex;
use crate::command::{self, Executor};
use std::string::ToString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::cell::RefCell;
use std::rc::Rc;
use std::mem;
use std::cmp::{PartialEq, Eq};
use std::ffi::OsStr;
use crate::tables::column::Column;

#[cfg(feature="arrowext")]
use datafusion::execution::context::ExecutionContext;

#[cfg(feature="arrowext")]
use datafusion::datasource::csv::{CsvFile, CsvReadOptions};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum CopyTarget {
    // Copies from server to client
    To,
    
    // Copies from client to server
    From
}

#[derive(Debug)]
enum CopyClient {

    // Results from a "copy to/from program 'prog'" command
    Program(String),
    
    // Results from a "copy to/from 'file'"
    File(String),
    
    // Copy to/from stdin/stdout
    Stdio
}

#[derive(Debug)]
pub struct Copy {

    // Copy to or from?
    target : CopyTarget,
    
    // Table string
    table : String,
    
    // Table columns target (if any)
    cols : Vec<String>,
    
    // Everything that goes in the 'with' clause.
    options : String,
    
    client : CopyClient,    
}

impl ToString for Copy {
    fn to_string(&self) -> String {
        let mut cp_s = format!("COPY {} ", self.table);
        if self.cols.len() > 0 {
            cp_s += "(";
            for (i, c) in self.cols.iter().enumerate() {
                cp_s += c;
                if i < self.cols.len() - 1 {
                    cp_s += ",";
                }
            }
            cp_s += ") ";
        }
        match self.target {
            CopyTarget::From => cp_s += "FROM STDIN",
            CopyTarget::To => cp_s += "TO STDOUT"
        }
        if self.options.len() > 0 {
            cp_s += " WITH ";
            cp_s += &self.options[..];
        }
        cp_s += ";";
        println!("Built copy statement: {}", cp_s);
        cp_s
    }
}

#[derive(Debug)]
enum AnyStatement {
    Parsed(Statement, String),
    Raw(String),
    Copy(Copy)
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

// Carries a result (arranged over columns)
#[derive(Debug, Clone)]
pub enum QueryResult {

    // Returns a valid executed query with its table represented over columns.
    Valid(String, Table),

    // Returns the result of a successful insert/update/delete statement.
    Statement(String),

    // Returns the result of a successful create/drop/alter statement.
    Modification(String),

    // Returns a query/statement rejected by the database engine, carrying its error message.
    Invalid(String)
}

// TODO check UTF-8 encoding. Getting error:
// thread 'main' panicked at 'byte index 64 is not a char boundary; it is inside 'ç' (bytes 63..65) of
// When using ç in a text field.

/*#[derive(Debug, Clone)]
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
}*/

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

/// Parse this query sequence, first splitting the token vector
/// at the semi-colons (delimiting statements) and then parsing
/// each statement individually. On error, the un-parsed statement is returned. 
/// Might fail globally if the tokenizer did not yield a valid token vector.
fn parse_sql_separate(sql : &str) -> Result<Vec<AnyStatement>, String> {
    let dialect = PostgreSqlDialect{};
    let mut tokenizer = Tokenizer::new(&dialect, &sql);
    let mut tokens = tokenizer.tokenize().map_err(|e| format!("{:?}", e) )?;
    let mut split_tokens : Vec<Vec<Token>> = Vec::new();
    let mut stmt_tokens : Option<Vec<Token>> = None;
    for tk in tokens.drain(0..) {
        // println!("new token = {}", tk);
        // println!("vec tokens = {:?}", stmt_tokens);
        match tk {
            Token::SemiColon => {
                if let Some(mut group) = stmt_tokens.take() {
                    group.push(Token::SemiColon);
                    split_tokens.push(group);
                } 
            },
            Token::Whitespace(Whitespace::SingleLineComment(_)) => { 
                // A single line comment starting a statement block
                // is preventing Parser::new(.) of returning a valid select statement,
                // so they are parsed away here.
            },
            other => {
                match stmt_tokens {
                    Some(ref mut stmt_tokens) => stmt_tokens.push(other),
                    None => stmt_tokens = Some(vec![other]),
                }
            }
        }
    }
    if let Some(last) = stmt_tokens {
        if last.len() > 0 {
            let non_whitespace = last.iter()
                .any(|tk| match tk {
                    Token::Whitespace(_) => false,
                    _ => true
                });
            if non_whitespace {
                split_tokens.push(last);
            }
        }
    }
    
    // Remove token groups which have only whitespaces
    for i in 0..split_tokens.len() {
        let all_ws = split_tokens[i].iter()
            .all(|tk| match tk { Token::Whitespace(_) => true, _ => false }); 
        if all_ws {
            split_tokens.remove(i);
        }
    }
    
    println!("Split tokens = {:?}", split_tokens);
    let mut any_stmts = Vec::new();
    for group in split_tokens {
        let mut orig = String::new();
        for tk in group.iter() {
            orig += &tk.to_string()[..];
        }
        println!("Recovered orig = {:?}", orig);
        // TODO group begin ... commit; together here, since we separated
        // tokens at ; before parsing.
        let mut parser = Parser::new(group);
        match parser.parse_statement() {
            Ok(stmt) => match stmt {
                Statement::Copy{ table_name, columns, .. } => {
                    // sqlparser::parse_copy is only accepting the copy (..) from stdin sequence.
                    let cols = columns.iter().map(|c| c.to_string()).collect();
                    any_stmts.push(AnyStatement::Copy(Copy { 
                        target : CopyTarget::From, 
                        cols, 
                        table : table_name.to_string(),
                        options : String::new(), 
                        client : CopyClient::Stdio
                     }));
                },
                other_stmt => {
                    any_stmts.push(AnyStatement::Parsed(other_stmt, orig));
                }
            },
            Err(e) => {
                match parse_copy(orig.clone()) {
                    Ok(copy) => {
                        any_stmts.push(AnyStatement::Copy(copy))
                    },
                    Err(copy_e) => {
                        println!("Sql parsing error = {}", e);
                        println!("Error parsing copy: {}", copy_e);
                    }
                }
                
            }
        }
    }
    Ok(any_stmts)
}

/// Modifies the iterator until the first non-whitespace token is found, returning it.
fn take_while_not_whitespace<'a, I>(token_iter : &mut I) -> Option<&'a Token>
where
    I : Iterator<Item=&'a Token>
{
    token_iter.next().and_then(|tk| match tk {
        Token::Whitespace(_) => take_while_not_whitespace(token_iter),
        other => Some(other)
    })
}

fn decide_table(token_iter : &mut std::slice::Iter<'_, Token>) -> Result<String, String> {
    if let Some(tk) = take_while_not_whitespace(token_iter) {
        match tk {
            Token::Word(w) => {
                Ok(w.value.to_string())
            },
            Token::LParen => {
                let mut tbl = String::from("(");
                while let Some(tk) = token_iter.next()  {
                    match tk {
                        Token::RParen => {
                            tbl += ")";
                            break;
                        },
                        tk => {
                            tbl += &tk.to_string();
                        }
                    }
                }
                Ok(tbl)
            },
            _ => Err(format!("Invalid table name"))
        }
    } else {
        Err(format!("Missing table name"))
    }
}

fn decide_target_keyword(w : &Word) -> Result<CopyTarget, String> {
    match w.keyword {
        Keyword::FROM => Ok(CopyTarget::From),
        Keyword::TO => Ok(CopyTarget::To),
        _ => return Err(format!("Unknown copy destination: {}", w))
    }
}

fn decide_target(
    token_iter : &mut std::slice::Iter<'_, Token>, 
    cols : &mut Vec<String>
) -> Result<CopyTarget, String> {
    match take_while_not_whitespace(token_iter) {
        Some(&Token::LParen) => {
            while let Some(tk) = token_iter.next()  {
                match tk {
                    Token::Word(w) => {
                        cols.push(w.value.to_string());
                    },
                    Token::RParen => {
                        break; 
                    },
                    _ => { }
                }
            }
            match take_while_not_whitespace(token_iter) {
                Some(Token::Word(w)) => {
                    decide_target_keyword(&w)
                },
                Some(other) => {
                    return Err(format!("Invalid target copy token: {}", other));
                },
                None => { 
                    return Err(format!("Missing copy target"));
                }
            }
        },
        Some(Token::Word(w)) => {
            decide_target_keyword(&w)
        },
        Some(other) => {
            return Err(format!("Invalid copy token: {}", other));
        },
        None => {
            return Err(format!("Empty copy destination"));
        }
    }
}

fn decide_client(
    token_iter : &mut std::slice::Iter<'_, Token>,
    target : &CopyTarget
) -> Result<CopyClient, String> {
    match take_while_not_whitespace(token_iter) {
        Some(Token::Word(w)) => {
            if &w.value[..] == "PROGRAM" || &w.value[..] == "program" {
                if let Some(tk) = take_while_not_whitespace(token_iter) {
                    match tk {
                        Token::SingleQuotedString(prog) => Ok(CopyClient::Program(prog.to_string())),
                        _ => return Err(format!("Invalid program string"))
                    }
                } else {
                    return Err(format!("Missing program string"));
                }
            } else {
                if w.keyword == Keyword::STDIN {
                    if *target == CopyTarget::From {
                        Ok(CopyClient::Stdio)
                    } else {
                        return Err(format!("Invalid copy client"));
                    }
                } else {
                    if &w.value[..] == "STDOUT" || &w.value[..] == "stdout" {
                        if *target == CopyTarget::To {
                            Ok(CopyClient::Stdio)
                        } else {
                            return Err(format!("Invalid copy client"));
                        }
                    } else {
                        return Err(format!("Invalid copy client"));
                    }
                }
            }
        },
        Some(Token::SingleQuotedString(file)) => {
            Ok(CopyClient::File(file.to_string()))
        },
        Some(other) => {
            return Err(format!("Invalid client copy specification: {}", other))
        },
        None => {
            return Err(format!("Missing copy destination"))
        }
    }
}

fn parse_options(token_iter : &mut std::slice::Iter<'_, Token>) -> String {
    let mut options = String::new();
    if let Some(Token::Word(w)) = take_while_not_whitespace(token_iter) {
        if w.keyword == Keyword::WITH {
            while let Some(tk) = token_iter.next() {
                match tk {
                    Token::Word(w) => {
                        options += &w.to_string()[..];
                        options += " ";
                    },
                    _ => { }
                }
            }
        }
    }
    options
}

/// Substitute copy statements in the query sequence string so they can
/// be correctly parsed by SqlParse and later sent to PostgreSQL via
/// copy to stdin/copy to stdout;
fn parse_copy(mut query : String) -> Result<Copy, String> {
    let copy_regx = Regex::new(
        r"(copy|COPY)\s+.*\s+(from|FROM|to|TO)\s+((program|PROGRAM)\s)?('.*'|\$\$.*\$\$|stdin|STDIN|stdout|STDOUT)(\s+with.*)?;"
    ).unwrap();
    let c_match = copy_regx.find(&query).ok_or(format!("Copy statement regex parsing error"))?;
    // println!("Found copy substitution: {:?}", c_match);
    let dialect = PostgreSqlDialect{};
    
    let whitespace_err = format!("Missing whitespace at copy statement");
    let is_whitespace = |tk : &Token| -> Result<(), String> {
        match tk {
            Token::Whitespace(_) => Ok(()),
            _ => Err(whitespace_err.clone())
        }
    };
    let mut tokenizer = Tokenizer::new(&dialect, &query[c_match.start()..c_match.end()]);
    let tokens = tokenizer.tokenize().map_err(|e| format!("{:?}", e) )?;
    // println!("Tokens = {:?}", tokens);
    
    let mut token_iter = tokens.iter();
    if let Some(Token::Word(w)) = take_while_not_whitespace(&mut token_iter) {
        if w.keyword != Keyword::COPY {
            return Err(format!("Invalid first word for copy statement"));
        }
    }
    let table = decide_table(&mut token_iter)?;
    let mut cols = Vec::new();
    let target : CopyTarget = decide_target(&mut token_iter, &mut cols)?;
    let client = decide_client(&mut token_iter, &target)?;
    let options = parse_options(&mut token_iter);
    
    Ok(Copy{ table, cols, client, target, options })
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

/// Parse a SQL String, separating the statements and verifying if they are a select
/// or another kind of query. Use this if no client-side parsing is desired.
/// TODO remove ; from text literals "" and $$ $$ when doing the
/// analysis. Returns true for each statement if the resulting statement
/// is a select. This is a crude fallible approach, used as fallbach when
/// sqlparser is unable to parse a query due to engine-specific SQL extensions.
pub fn split_sql(sql_text : String) -> Result<Vec<(String, bool)>, String> {
    let stmts_strings : Vec<_> = sql_text.split(";")
        .filter(|c| c.len() > 0 && *c != "\n" && *c != " " && *c != "\t")
        .map(|c| c.trim_start().trim_end().to_string()).collect();
    let mut stmts = Vec::new();
    // TODO this will break if string literals contain select/with statements within them.
    for stmt in stmts_strings {
        println!("{}", stmt);
        let is_select = stmt.starts_with("select") || stmt.starts_with("SELECT") ||
            (stmt.starts_with("with") && (stmt.contains("select") || stmt.contains("SELECT"))) ||
            (stmt.starts_with("WITH") && (stmt.contains("select") || stmt.contains("SELECT"))) ||
            stmt.starts_with("pragma") || stmt.starts_with("PRAGMA");
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
    PostgreSql{conn_str : String, conn : postgres::Client, exec : Arc<Mutex<(Executor, String)>> },
    Sqlite3{path : Option<PathBuf>, conn : rusqlite::Connection},

    #[cfg(feature="arrowext")]
    Arrow{ ctx : ExecutionContext }
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
            Ok(conn) => Ok(SqlEngine::PostgreSql{ 
                conn_str, 
                conn, 
                exec : Arc::new(Mutex::new((Executor::new(), String::new()))) 
            }),
            Err(e) => Err(e.to_string())
        }
    }

    pub fn remove_sqlite3_udfs(&self, loader : &FunctionLoader, lib_name : &str) {
        match self {
            SqlEngine::Sqlite3{ conn, .. } => {
                for f in loader.fn_list_for_lib(lib_name) {
                    if let Err(e) = conn.remove_function(&f.name, f.args.len() as i32) {
                        println!("{}", e);
                    }
                }
            },
            _ => println!("No UDFs can be registered with the current engine")
        }
    }

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
    // Libraries that are not active but are loaded stay on main memory, but will not
    // be registered by this function because load_functions return only active libraries.
    // Perhaps only let the user add/remove/active libraries when there is no connection open
    // for safety.
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
                        LoadedFunc::I32(f) => {
                            let raw_fn = unsafe { f.into_raw() };
                            conn.create_scalar_function(
                                &func.name,
                                n_arg,
                                FunctionFlags::empty(),
                                move |ctx| { unsafe{ raw_fn(ctx) } }
                            )
                        },
                        LoadedFunc::F64(f) => {
                            let raw_fn = unsafe { f.into_raw() };
                            conn.create_scalar_function(
                                &func.name,
                                n_arg,
                                FunctionFlags::empty(),
                                move |ctx| { unsafe{ raw_fn(ctx) } }
                            )
                        },
                        LoadedFunc::Text(f) => {
                            let raw_fn = unsafe { f.into_raw() };
                            conn.create_scalar_function(
                                &func.name,
                                n_arg,
                                FunctionFlags::empty(),
                                move |ctx| { unsafe{ raw_fn(ctx) } }
                            )
                        },
                        LoadedFunc::Bytes(f) => {
                            let raw_fn = unsafe { f.into_raw() };
                            conn.create_scalar_function(
                                &func.name,
                                n_arg,
                                FunctionFlags::empty(),
                                move |ctx| { unsafe{ raw_fn(ctx) } }
                            )
                        }
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
                        rusqlite::vtab::csvtab::load_module(&conn)?;
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
                            if let Err(e) = self.try_run(q, true, /*None*/ ) {
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

    /// Get all SQLite table names.
    /// TODO This will break if there is a table under the temp schema with the same name
    /// as a table under the global schema.
    fn get_sqlite_tbl_names(&mut self) -> Option<Vec<String>> {
        let tbl_query = String::from("select name from sqlite_master where type = 'table' union \
            select name from temp.sqlite_master where type = 'table';");
        // select * from temp.sqlite_master;
        let ans = self.try_run(tbl_query, false)
            .map_err(|e| println!("{}", e) ).ok()?;
        if let Some(q_res) = ans.get(0) {
            match q_res {
                QueryResult::Valid(_, names) => {
                    names.get_column(0).and_then(|c| {
                        let s : Option<Vec<String>> = c.clone().try_into().ok();
                        s
                    })
                },
                QueryResult::Invalid(msg) => { println!("{}", msg); None },
                _ => None
            }
        } else {
            println!("Query for DB info did not yield any results");
            None
        }
    }

    /// col_types might be an empty string here because sqlite3 does not require
    /// that the types for all columns are declared. We treat the type as unknown in this case.
    fn pack_column_types(
        col_names : Vec<String>,
        col_types : Vec<String>
    ) -> Option<Vec<(String, DBType)>> {
        if col_names.len() != col_types.len() {
            println!("Column names different than column types length");
            return None;
        }
        let mut types = Vec::new();
        for ty in col_types {
            if let Ok(t) = ty.parse::<DBType>() {
                types.push(t);
            } else {
                println!("Unable to parse type: {:?}", ty);
                return None;
            }
        }
        let cols : Vec<(String, DBType)> = col_names.iter()
            .zip(types.iter())
            .map(|(s1, s2)| (s1.clone(), s2.clone() ))
            .collect();
        Some(cols)
    }

    fn get_sqlite_columns(&mut self, tbl_name : &str) -> Option<DBObject> {
        let col_query = format!("pragma table_info({});", tbl_name);
        let ans = self.try_run(col_query, false).map_err(|e| println!("{}", e) ).ok()?;
        let q_res = ans.get(0)?;
        match q_res {
            QueryResult::Valid(_, col_info) => {
                let names = col_info.get_column(1)
                    .and_then(|c| { let s : Option<Vec<String>> = c.clone().try_into().ok(); s })?;
                println!("{:?}", col_info.get_column(2));
                let col_types = col_info.get_column(2)
                    .and_then(|c| match c {
                        Column::Nullable(n) => {
                            let opt_v : Option<Vec<Option<String>>> = n.as_ref().clone().try_into().ok();
                            match opt_v {
                                Some(v) => {
                                    let v_flat = v.iter()
                                        .map(|s| s.clone().unwrap_or(String::new()))
                                        .collect::<Vec<String>>();
                                    Some(v_flat)
                                },
                                None => None
                            }
                        },
                        _ => {
                            let s : Option<Vec<String>> = c.clone().try_into().ok();
                            s
                        } 
                    })?;
                let cols = Self::pack_column_types(names, col_types)?;
                let obj = DBObject::Table{ name : tbl_name.to_string(), cols };
                Some(obj)
            },
            QueryResult::Invalid(msg) => { println!("{}", msg); None },
            _ => None
        }
    }

    fn get_postgre_extensions(&mut self) {
        // First, check all available extensions.
        let ext_query = "select extname::text, extversion from pg_available_extensions;";

        // Then, add (installed) tag to those that also appear here:
        let used_query = "select extname::text from pg_extension;";
    }

    fn get_postgre_roles(&mut self) {
        // let role_query = "select rolinherit, rolcanlogin, rolsuper, from pg_catalog.pg_roles";
    }

    // pg_proc.prokind codes: f = function; p = procedure; a = aggregate; w = window
    fn get_postgre_functions(&mut self) {
        let fn_query = r#"
            with arguments as (
                with arg_types as (select pg_proc.oid as proc_oid,
                    unnest(proallargtypes) as arg_oid
                    from pg_catalog.pg_proc
                ) select arg_types.proc_oid as proc_id, array_agg(cast(typname as text)) as arg_typename
                    from pg_catalog.pg_type inner join arg_types on pg_type.oid = arg_types.arg_oid
                    group by arg_types.proc_oid
                    order by arg_types.proc_oid
            ) select pg_proc.oid,
                pg_proc.prokind,
                proname::text,
                arguments.arg_typename,
                cast(typname as text) as ret_typename
            from pg_catalog.pg_proc inner join pg_catalog.pg_type on pg_proc.prorettype = pg_type.oid
                inner join arguments on pg_proc.oid = arguments.proc_id
            order by pg_proc.oid;"#;
    }

    /// Return HashMap of Schema->Tables
    fn get_postgre_schemata(&mut self) -> Option<HashMap<String, Vec<String>>> {
        let tbl_query = String::from("select schemaname::text, tablename::text \
            from pg_catalog.pg_tables \
            where schemaname != 'pg_catalog' and schemaname != 'information_schema';");
        let ans = self.try_run(tbl_query, false)
            .map_err(|e| println!("{}", e) ).ok()?;
        let q_res = ans.get(0)?;
        match q_res {
            QueryResult::Valid(_, tables) => {
                let schemata = tables.get_column(0).and_then(|c| {
                    let s : Option<Vec<String>> = c.clone().try_into().ok();
                    s
                });
                let names = tables.get_column(1).and_then(|c| {
                    let s : Option<Vec<String>> = c.clone().try_into().ok();
                    s
                });
                if let Some(schemata) = schemata {
                    if let Some(names) = names {
                        let mut schem_hash = HashMap::new();
                        for (schema, table) in schemata.iter().zip(names.iter()) {
                            let tables = schem_hash.entry(schema.clone()).or_insert(Vec::new());
                            tables.push(table.clone());
                        }
                        Some(schem_hash)
                    } else {
                        println!("Could not load table names to String vector");
                        None
                    }
                } else {
                    println!("Could not load schema column to String vector");
                    None
                }
            },
            QueryResult::Invalid(msg) => { println!("{}", msg); None },
            _ => None
        }
    }

    // TODO get PK/FK constraints
    // select * from information_schema.constraint_column_usage where table_name='experiment';

    fn get_postgre_columns(&mut self, schema_name : &str, tbl_name : &str) -> Option<DBObject> {
        let col_query = format!("select column_name::text,data_type::text \
            from information_schema.columns where table_name = '{}' and table_schema='{}';", tbl_name, schema_name);
        let ans = self.try_run(col_query, false).map_err(|e| println!("{}", e) ).ok()?;
        if let Some(q_res) = ans.get(0) {
            match q_res {
                QueryResult::Valid(_, col_info) => {
                    let names = col_info.get_column(0)
                        .and_then(|c| { let s : Option<Vec<String>> = c.clone().try_into().ok(); s })?;
                    let col_types = col_info.get_column(1)
                        .and_then(|c| { let s : Option<Vec<String>> = c.clone().try_into().ok(); s })?;
                    let cols = Self::pack_column_types(names, col_types)?;
                    let obj = DBObject::Table{ name : tbl_name.to_string(), cols };
                    Some(obj)
                },
                QueryResult::Invalid(msg) => { println!("{}", msg); None },
                _ => None
            }
        } else {
            println!("Database info query did not return any results");
            None
        }
    }

    /// Copies from the PostgreSQL server into a client
    fn copy_pg_to(client : &mut postgres::Client, action : &Copy) -> Result<String, String> {
        let mut reader = client.copy_out(&action.to_string()[..])
            .map_err(|e| format!("{}", e) )?;
        let mut data = String::new();
        reader.read_to_string(&mut data).map_err(|e| format!("{}", e))?;
        Ok(data)
    }
    
    /// Copies from a client into the PostgreSQL server
    fn copy_pg_from(client : &mut postgres::Client, action : &Copy, data : &str) -> Result<u64, String> {
        let mut writer = client.copy_in(&action.to_string()[..])
            .map_err(|e| format!("{}", e) )?;
        writer.write_all(data.as_bytes()).map_err(|e| format!("{}", e))?;
        let n = writer.finish().map_err(|e| format!("{}", e) )?;
        Ok(n)
    }
    
    pub fn copy(conn : &mut postgres::Client, action : &Copy, exec : &Arc<Mutex<(Executor, String)>>) -> Result<u64, String> {
        match action.target {
            CopyTarget::From => {
                let csv_input = match &action.client {
                    CopyClient::Stdio => {
                        let mut executor = exec.lock().map_err(|e| format!("{}", e))?;
                        if executor.1.len() > 0 {
                            mem::take(&mut executor.1)
                        } else {
                            return Err(format!("No data cached in stdin"));
                        }
                    },
                    CopyClient::File(path) => {
                        let mut f = File::open(path).map_err(|e| format!("{}", e))?;
                        let mut content = String::new();
                        f.read_to_string(&mut content).map_err(|e| format!("{}", e))?;
                        if content.len() == 0 {
                            return Err(format!("File is empty"));
                        }
                        content
                    },
                    CopyClient::Program(p) => {
                        let mut executor = exec.lock().map_err(|e| format!("{}", e))?;
                        let input = mem::take(&mut executor.1);
                        if input.len() == 0 {
                            executor.0.queue_command(p.clone(), None);
                        } else {
                            executor.0.queue_command(p.clone(), Some(input));
                        }
                        let mut content = String::new();
                        executor.0.wait_result(|out| {
                            if out.status {
                                if out.txt.len() > 0 {
                                    content = out.txt;
                                    Ok(())
                                } else {
                                    Err(format!("Program standard output is empty"))
                                }
                            } else {
                                Err(format!("Command execution failed: {}", out.txt))
                            }
                        })?;
                        println!("Captured into stdout: {}", content);
                        content
                    }
                };
                Self::copy_pg_from(conn, &action, &csv_input)
            },
            CopyTarget::To => {
                let csv_out = Self::copy_pg_to(conn, &action)?;
                println!("Received data: {}", csv_out);
                if csv_out.len() == 0 {
                    return Err(format!("'COPY TO' returned no data"));
                }
                match &action.client {
                    CopyClient::Stdio => {
                        let mut executor = exec.lock().map_err(|e| format!("{}", e))?;
                        if executor.1.len() > 0 {
                            println!("Clearing previous data cache");
                            executor.1.clear();
                        }
                        executor.1 = csv_out.clone();
                    },
                    CopyClient::File(path) => {
                        if Path::new(&path).extension() != Some(&OsStr::new("csv")) {
                            return Err(format!("Path must point to csv file"));
                        }
                        let mut f = File::create(path).map_err(|e| format!("{}", e))?;
                        f.write_all(csv_out.as_bytes()).map_err(|e| format!("{}", e))?;
                    },
                    CopyClient::Program(p) => {
                        let mut cmd_out = String::new();
                        let mut executor = exec.lock().map_err(|e| format!("{}", e))?;
                        executor.0.queue_command(p.clone(), Some(csv_out.clone()));
                        executor.0.wait_result(|out| {
                            if out.status {
                                if out.txt.len() > 0 {
                                    cmd_out = out.txt;
                                }
                                Ok(())
                            } else {
                                Err(format!("Command execution failed: {}", out.txt))
                            }
                        })?;
                        if cmd_out.len() > 0 {
                            if executor.1.len() > 0 {
                                println!("Clearing previous data cache");
                                executor.1.clear();
                            }
                            executor.1 = cmd_out;
                        }
                    }
                }
                Ok(0)
            }
        }
    }
    
    pub fn get_db_info(&mut self) -> Option<Vec<DBObject>> {
        let mut top_objs = Vec::new();
        match &self {
            SqlEngine::Sqlite3{path : _, conn : _} => {
                if let Some(names) = self.get_sqlite_tbl_names() {
                    for name in names {
                        if let Some(obj) = self.get_sqlite_columns(&name) {
                            top_objs.push(obj);
                        } else {
                            println!("Failed to retrieve columns for table {}", name);
                            return None;
                        }
                    }
                } else {
                    println!("Could not get SQLite table names");
                    return None;
                }
                Some(top_objs)
            },
            SqlEngine::PostgreSql{..} => {
                if let Some(schemata) = self.get_postgre_schemata() {
                    println!("Obtained schemata: {:?}", schemata);
                    for (schema, tbls) in schemata.iter() {
                        let mut tbl_objs = Vec::new();
                        for t in tbls.iter() {
                            if let Some(tbl) = self.get_postgre_columns(&schema[..], &t[..]) {
                                tbl_objs.push(tbl);
                            } else {
                                println!("Failed getting columns for {}", t);
                                return None;
                            }
                        }
                        top_objs.push(DBObject::Schema{ name : schema.to_string(), children : tbl_objs });
                    }
                    Some(top_objs)
                } else {
                    println!("Failed retrieving database schemata");
                    None
                }
            },
            _ => None
        }
    }

    /*/// Table is an expesive data structure, so we pass ownership to the function call
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
    }*/

    /// After the statement execution status returned from the SQL engine,
    /// build a message to display to the user.
    fn build_statement_result(any_stmt : &AnyStatement, n : usize) -> QueryResult {
        match any_stmt {
            AnyStatement::Parsed(stmt, _) => match stmt {
                Statement::CreateView{..} => QueryResult::Modification(format!("Create view")),
                Statement::CreateTable{..} | Statement::CreateVirtualTable{..} => {
                    QueryResult::Modification(format!("Create table"))
                },
                Statement::CreateIndex{..} => QueryResult::Modification(format!("Create index")),
                Statement::CreateSchema{..} => QueryResult::Modification(format!("Create schema")),
                Statement::AlterTable{..} => QueryResult::Modification(format!("Alter table")),
                Statement::Drop{..} => QueryResult::Modification(format!("Drop table")),
                Statement::Copy{..} => QueryResult::Modification(format!("Copy")),
                _ => QueryResult::Statement(format!("{} row(s) modified", n))
            },
            AnyStatement::Raw(s) => {
                if s.contains("create table") || s.contains("CREATE TABLE") {
                    return QueryResult::Modification(format!("Create table"));
                }
                if s.contains("create virtual table") || s.contains("CREATE VIRTUAL TABLE") {
                    return QueryResult::Modification(format!("Create table"));
                }
                if s.contains("create temporary table") || s.contains("CREATE TEMPORARY TABLE") {
                    return QueryResult::Modification(format!("Create table"));
                }
                if s.contains("drop table") || s.contains("DROP TABLE") {
                    return QueryResult::Modification(format!("Drop table"));
                }
                if s.contains("alter table") || s.contains("ALTER TABLE") {
                    return QueryResult::Modification(format!("Alter table"));
                }
                if s.contains("create schema") || s.contains("CREATE SCHEMA") {
                    return QueryResult::Modification(format!("Create schema"));
                }
                QueryResult::Statement(format!("{} row(s) modified", n))
            },
            AnyStatement::Copy(c) => {
                unimplemented!()
            }
        }
    }

    fn append_relation(t_expr : &TableFactor, out : &mut String) {
        match t_expr {
            TableFactor::Table{ name, .. } => {
                if !out.is_empty() {
                    *out += " : ";
                }
                *out += &name.to_string();
            },
            TableFactor::Derived{ .. } | TableFactor::NestedJoin(_) => {

            }
        }
    }

    fn table_name_from_sql(sql : &str) -> Option<(String, String)> {
        let dialect = PostgreSqlDialect{};
        let ast = Parser::parse_sql(&dialect, sql).ok()?;
        if let Some(Statement::Query(q)) = ast.get(0) {
            if let SetExpr::Select(s) = &q.body {
                println!("{:?}", s);
                let mut from_names = String::new();
                let mut relation = String::new();
                for t_expr in s.from.iter() {
                    Self::append_relation(&t_expr.relation, &mut from_names);
                    for join in t_expr.joins.iter() {
                        Self::append_relation(&join.relation, &mut from_names);
                        if relation.is_empty() {
                            match join.join_operator {
                                JoinOperator::Inner(_) => relation += "inner",
                                JoinOperator::LeftOuter(_) => relation += "left",
                                JoinOperator::RightOuter(_) => relation += "right",
                                JoinOperator::FullOuter(_) => relation += "full",
                                _ => { }
                            }
                        }
                    }
                }
                println!("Name: {:?}", from_names);
                println!("Relation: {:?}", relation);
                Some((from_names, relation))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn query_postgre(conn : &mut postgres::Client, q : &str) -> QueryResult {
        match conn.query(q, &[]) {
            Ok(rows) => {
                match postgre::build_table_from_postgre(&rows[..]) {
                    Ok(mut tbl) => {
                        if let Some((name, relation)) = Self::table_name_from_sql(q) {
                            tbl.set_name(Some(name));
                            if !relation.is_empty() {
                                tbl.set_relation(Some(relation));
                            }
                        }
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
                            Ok(mut tbl) => {
                                if let Some((name, relation)) = Self::table_name_from_sql(q) {
                                    tbl.set_name(Some(name));
                                    if !relation.is_empty() {
                                        tbl.set_relation(Some(relation));
                                    }
                                }
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

    // TODO postgres will panick if the user pass any $1 argument, since it will be interpreted
    // as a parameter to the empty slice.
    fn exec_postgre(conn : &mut postgres::Client, stmt : &AnyStatement) -> QueryResult {
        let ans = match stmt {
            AnyStatement::Parsed(stmt, s) => {
                let s = format!("{}", stmt);
                conn.execute(&s[..], &[])
            },
            AnyStatement::Raw(s) => conn.execute(&s[..], &[]),
            AnyStatement::Copy(_) => { 
                unimplemented!()
            }
        };
        match ans {
            Ok(n) => Self::build_statement_result(&stmt, n as usize),
            Err(e) => QueryResult::Invalid(e.to_string())
        }
    }

    fn exec_sqlite(conn : &mut rusqlite::Connection, stmt : &AnyStatement) -> QueryResult {
        let ans = match stmt {
            AnyStatement::Parsed(stmt, s) => {
                let s = format!("{}", stmt);
                conn.execute(&s[..], rusqlite::NO_PARAMS)
            },
            AnyStatement::Raw(s) => conn.execute(&s[..], rusqlite::NO_PARAMS),
            AnyStatement::Copy(_) => unimplemented!()
        };
        match ans {
            Ok(n) => Self::build_statement_result(&stmt, n),
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
                SqlEngine::PostgreSql{ conn_str : _ , conn, exec : _ } => {
                    if is_select {
                        results.push(Self::query_postgre(conn, &format!("{}", stmt)));
                    } else {
                        results.push(Self::exec_postgre(conn, &AnyStatement::Raw(format!("{}", stmt))));
                    }
                },
                SqlEngine::Sqlite3{ path : _, conn} | SqlEngine::Local{ conn } => {
                    if is_select {
                        results.push(Self::query_sqlite(conn, &format!("{}", stmt)));
                    } else {
                        results.push(Self::exec_sqlite(conn, &AnyStatement::Raw(format!("{}", stmt))));
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

    /*fn substitute_macros() {
        let var_re = Regex::new(r"\$\(.*\)").unwrap();
        let cmd_re = Regex::new(r"\$\{.*\}").unwrap();
        let var_matches = var_re.find_iter(&query_seq).collect::<Vec<_>>();
        let cmd_matches = cmd_re.find_iter(&query_seq).collect::<Vec<_>>();
        println!("Found variable substitutions: {:?}", var_matches);
        println!("Found command substitutions: {:?}", cmd_matches);
    }*/
    
    /// It is important that every time this method is called,
    /// at least one query result is pushed into the queue, or else
    /// the GUI will be insensitive waiting for a response.
    pub fn try_run(
        &mut self,
        query_seq : String,
        parse : bool
    ) -> Result<Vec<QueryResult>, String> {
    
        // Substitute $() (variable) and ${} (command) macros before parsing the SQL.    
        // let (query_seq, copies) = Self::substitute_copies(query_seq)?; 
        // println!("Captured copies: {:?}", copies);
        let stmts = match parse {
            true => match parse_sql_separate(&query_seq) {
                Ok(stmts) => stmts,
                Err(e) => {
                    println!("Parsing error: {}", e);
                    return self.run_any(query_seq);
                }
            }
            false => return self.run_any(query_seq)
        };
        let mut results = Vec::new();
        if stmts.len() == 0 {
            return Err(String::from("Empty query sequence"));
        }
        println!("Statements = {:?}", stmts);
        
        // Copies are parsed and executed at client-side. It is important to 
        // give just the copy feedback when we have only copies, but we give
        // a statement feedback otherwise.
        let mut all_copies = stmts.iter().all(|stmt| match stmt {
            AnyStatement::Copy(_) => true,
            _ => false
        });
        match self {
            SqlEngine::Inactive => { return Err(String::from("Inactive Sql engine")); },
            SqlEngine::PostgreSql{ conn_str : _ , ref mut conn, ref mut exec } => {
                for any_stmt in stmts {
                    // let (stmt, opt_sub) = filter_single_function_out(&stmt);
                    // println!("Parsed statement: {}", stmt_string);
                    match any_stmt {
                        AnyStatement::Parsed(stmt, query) => match stmt {
                            Statement::Query(q) => {
                                results.push(Self::query_postgre(conn, &format!("{}", q)));
                            },
                            stmt => {
                                results.push(Self::exec_postgre(conn, &AnyStatement::Parsed(stmt.clone(), format!("{}", stmt))));
                            }
                        },
                        AnyStatement::Copy(c) => {
                            println!("Found copy: {:?}", c);
                            match Self::copy(conn, &c, &*exec) {
                                Ok(n) => match (c.target, n) {
                                    (CopyTarget::From, 0) => {
                                        results.push(QueryResult::Invalid(format!("No rows copied to server")));
                                    },
                                    (CopyTarget::From, n) => {
                                        results.push(QueryResult::Statement(format!("Copied {} row(s)", n)));        
                                    },
                                    (CopyTarget::To, _) => {
                                        results.push(QueryResult::Statement(format!("Copy to client successful")));        
                                    }
                                },
                                Err(e) => {
                                    results.push(QueryResult::Invalid(e));
                                }
                            }
                        },
                        AnyStatement::Raw(r) => {
                            unimplemented!()
                        }
                    }
                }
            },
            SqlEngine::Sqlite3{ path : _, conn} | SqlEngine::Local{ conn } => {
                for any_stmt in stmts {
                    match any_stmt {
                        AnyStatement::Parsed(stmt, query) => match stmt {
                            Statement::Query(q) => {
                                // println!("Sending query: {}", q);
                                results.push(Self::query_sqlite(conn, &format!("{}", q)));
                            },
                            stmt => {
                                results.push(Self::exec_sqlite(conn, &AnyStatement::Parsed(stmt.clone(), format!("{}", stmt))));
                            }
                        },
                        AnyStatement::Copy(c) => {
                            println!("Found copy: {:?}", c);
                            //Self::copy(&c, &exec)?;
                            unimplemented!()
                        },
                        AnyStatement::Raw(r) => {
                            unimplemented!()
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

    pub fn launch( /*loader : Arc<Mutex<FunctionLoader>>*/ ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<(String, bool)>();
        let (ans_tx, ans_rx) = mpsc::channel::<Vec<QueryResult>>();

        let engine = Arc::new(Mutex::new(SqlEngine::Inactive));
        let engine_c = engine.clone();

        // Must join on structure desctruction.
        let r_thread = thread::spawn(move ||  {
            //let loader = loader.clone();
            loop {
                // TODO perhaps move SQL parsing to here so loader is passed to
                // try_run iff there are local functions matching the query.
                match (cmd_rx.recv(), engine_c.lock() /*, loader.lock()*/ ) {
                    (Ok((cmd, parse)), Ok(mut eng) /*, Ok(loader)*/ ) => {
                        let result = eng.try_run(cmd, parse, /*Some(&loader)*/ );
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
            self.clear_results();
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

    pub fn clear_results(&self) {
        while let Ok(mut res) = self.ans_receiver.try_recv() {
            let _ = res;
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




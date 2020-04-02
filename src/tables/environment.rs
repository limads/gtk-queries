use nalgebra::DVector;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::PathBuf;
use nalgebra::*;
use nalgebra::base::storage::Storage;
//use rgsl::types::{VectorF64, VectorF32};
//use utf8mat::csv;
//pub mod sql;
//pub mod button;
use super::csv::*;
//pub mod sql_integration;
//pub mod table_notebook;
//pub mod table_widget;
//pub mod column;
//use column::*;
//use nlearn::column::*;
//mod numeric_utils;
use crate::tables::sql::*;
use chrono;
//use nlearn::table::*;
//pub mod table;
//use table::*;
//pub mod environment_source;
use super::source::*;
//pub mod stdin;
use super::stdin::*;
use std::path::Path;
use super::sql::*;
use super::table::*;
//pub mod decoding;
//use decoding::*;
//pub mod extension;
//use extension::*;

// env_source::new_from_path
// env_source::new_from_sql_conn

pub struct TableEnvironment {
    source : EnvironmentSource,
    listener : SqlListener,
    tables : Vec<Table>,
    last_update : Option<String>
}

/*enum TableKind {
    Named(HashMap<String, Column>),
    Unnamed(DMatrix<f32>)
}*/

/*pub struct TableDataSource {
    // Holds all data a plot can use at any given point.
    content : HashMap<String, Vec<String>>,

    // A subset of the keys in content actually used by the plot.
    // Those columns should be able to be parsed to f64 so they can
    // be displayed by the plot
    used_cols : Vec<String>,

    num_cols : HashMap<String, Vec<f64>>

    //plot    : Rc<RefCell<PlotView>>
}*/

impl TableEnvironment {

    /*pub fn new() -> TableDataSource {
        TableDataSource{
            content : HashMap::new(),
            used_cols : Vec::new(),
            num_cols : HashMap::new()
        }
    }*/

    pub fn new(src : EnvironmentSource) -> Self {
        Self{
            source : src,
            listener : SqlListener::launch(),
            tables : Vec::new(),
            last_update : None
        }
    }

    pub fn set_new_postgre_engine(
        &mut self,
        conn_str : String
    ) -> Result<(), String> {
        match SqlEngine::try_new_postgre(conn_str) {
            Ok(engine) => { self.update_engine(engine)?; Ok(()) },
            Err(msg) => Err(msg)
        }
    }

    pub fn set_new_sqlite3_engine(
        &mut self,
        path : Option<PathBuf>
    ) -> Result<(), String> {
        match SqlEngine::try_new_sqlite3(path) {
            Ok(engine) => { self.update_engine(engine)?; Ok(()) },
            Err(msg) => Err(msg)
        }
    }

    pub fn disable_engine(&mut self) {
        if let Ok(mut engine) = self.listener.engine.lock() {
            *engine = SqlEngine::Inactive;
        } else {
            println!("Could not acquire lock over SQL engine");
        }
    }

    fn update_engine(&mut self, engine : SqlEngine) -> Result<(), String> {
        if let Ok(mut old_engine) = self.listener.engine.lock() {
            *old_engine = engine;
            Ok(())
        } else {
            Err("Error acquiring lock over engine when updating it".into())
        }
    }

    pub fn get_engine_name(&self) -> String {
        if let Ok(engine) = self.listener.engine.lock() {
            match *engine {
                SqlEngine::Inactive => String::from("Inactive"),
                SqlEngine::PostgreSql{..} => String::from("PostgreSQL"),
                SqlEngine::Sqlite3{..} => String::from("SQLite3"),
                SqlEngine::Local{..} => String::from("Local"),
            }
        } else {
            String::from("Unavailable")
        }
    }

    /// Get engine active state. Consider it active in the event
    /// the lock could not be acquired.
    pub fn is_engine_active(&self) -> bool {
        if let Ok(engine) = self.listener.engine.lock() {
            match *engine {
                SqlEngine::Inactive => false,
                _ => true
            }
        } else {
            println!("Warning : Could not acquire lock over engine");
            true
        }
    }

    pub fn get_last_update_date(&self) -> String {
        match &self.last_update {
            Some(date) => date.clone(),
            None => String::from("Unknown")
        }
    }

    pub fn send_current_query(&mut self) {
        //println!("{:?}", self.source);
        let query = match self.source {
            EnvironmentSource::PostgreSQL(ref db_pair) =>{
                Some(db_pair.1.clone())
            },
            EnvironmentSource::SQLite3(ref db_pair) => {
                Some(db_pair.1.clone())
            },
            _ => None
        };
        if let Some(q) = query {
            self.listener.send_command(q);
        } else {
            println!("No query available to send.");
        }
    }

    pub fn clear_queries(&mut self) {
        let no_query = String::new();
        self.prepare_query(no_query);
    }

    pub fn prepare_query(&mut self, sql : String) {
        match self.source {
            EnvironmentSource::PostgreSQL((_, ref mut q)) => {
                *q = sql;
            },
            EnvironmentSource::SQLite3((_, ref mut q)) => {
                *q = sql;
            },
            _ => { }
        }
    }

    pub fn prepare_and_send_query(&mut self, sql : String) {
        //self.listener.send_command(sql.clone());
        self.prepare_query(sql);
        self.send_current_query();
    }

    /// Try to update the tables, potentially returning the first error
    /// message encountered by the database. Returns None if there
    /// is no update; Returns the Ok(result) if there is update, potentially
    /// carrying the first error the database encountered.
    pub fn maybe_update_from_query_results(
        &mut self
    ) -> Option<Result<(),String>> {
        let results = self.listener.maybe_get_result()?;
        self.tables.clear();
        for r in results {
            match r {
                QueryResult::Valid(tbl) => self.tables.push(tbl),
                QueryResult::Invalid(msg) => {
                    return Some(Err(msg));
                },
                QueryResult::Statement(_) => {
                    //return Some(Err(String::from("Last")))
                }
            }
        }
        self.last_update = Some(format!("{}", chrono::offset::Local::now()));
        Some(Ok(()))
    }

    pub fn result_last_statement(&self) -> Option<Result<String,String>> {
        let results = self.listener.maybe_get_result()?;
        if let Some(r) = results.last() {
            println!("{:?}", r);
            match r {
                QueryResult::Statement(s) => Some(Ok(s.clone())),
                QueryResult::Invalid(e) => Some(Err(e.clone())),
                QueryResult::Valid(_) => None
            }
        } else {
            None
        }
    }

    /// Try to update the table from a source such as a SQL connection string
    /// or a file path.
    pub fn update_source(
        &mut self, src : EnvironmentSource, clear : bool
    ) -> Result<(),String> {
        if clear {
            self.tables.clear();
        }
        //println!("{:?}", src );
        match src.clone() {
            // Updates from a single CSV file. This implies only a single
            // table is available.
            EnvironmentSource::File(path, content) => {
                //println!("Received source at update_from_source: {}", content);
                self.tables.clear();
                let p = Path::new(&path);
                let p = p.file_stem().ok_or("Could not extract table name from path".to_string())?;
                let tbl_name = Some(p.to_str().ok_or("Could not convert table path to str".to_string())?.to_string());
                match Table::new_from_text(content.to_string()) {
                    Ok(tbl) => { self.tables.push(tbl)  },
                    Err(e) => { return Err(format!("Error: {}", e)); }
                }
            },
            EnvironmentSource::PostgreSQL((conn, q)) => {
                self.set_new_postgre_engine(conn)
                    .map_err(|e| { format!("{}", e) })?;
                if !q.is_empty() {
                    self.prepare_and_send_query(q);
                }
            },
            EnvironmentSource::SQLite3((conn, q)) => {
                self.set_new_sqlite3_engine(conn)
                    .map_err(|e|{ format!("{}", e) })?;
                if !q.is_empty() {
                    self.prepare_and_send_query(q);
                }
            },
            _ => { println!("Invalid_source"); }
        }
        self.source = src;
        Ok(())
    }

    pub fn convert_source_to_in_memory_sqlite(&mut self) {
        match &self.source {
            EnvironmentSource::SQLite3(_) => {
                println!("Source is already SQLite3");
            },
            EnvironmentSource::PostgreSQL(_) => {
                println!("Invalid source update: PostgreSQL->SQLite3");
            },
            _ => {
                if self.tables.len() == 0 {
                    println!("No tables on environment for conversion to in-memory SQLite3");
                    return;
                }
                let new_src = EnvironmentSource::SQLite3((None,"".into()));
                let tables = self.tables.clone();
                self.clear();
                match self.update_source(new_src, true) {
                    Ok(_) => {
                        if let Ok(mut engine) = self.listener.engine.lock() {
                            for t in tables {
                                engine.insert_external_table(&t);
                            }
                        } else {
                            println!("Unable to acquire lock over SQL listener to insert tables");
                        }
                    },
                    Err(e) => println!("{}", e)
                }
            }
        }
    }

    /*pub fn show_all_cols(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for t in &self.tables {
            let t_keys : Vec<String> = t.content.iter()
                .map(|c| c.name.to_string())
                .collect();
            keys.extend(t_keys);
        }
        keys
    }*/

    /// This is the main entry point for updating the plot data.
    /// Internally, this function updates the numeric representation
    /// of the used columns, and call queue_draw on the maintained
    /// reference to the plot.
    /*pub fn update_used_cols(&mut self, cols : Vec<String>)
    -> Result<(), &str> {
        self.used_cols = cols;

        for col in self.used_cols.iter() {
            if let None = self.show_all_cols().iter()
                .find(|c| { *c == col} ) {
                return Err("Column not present");
            }
        }
        Ok(())
        // self.update_data()
    }*/

    /// If a valid path for a csv file is informed and the
    /// content can be parsed, fill the inner content with
    /// the CSV values.
    /*pub fn update_content_from_csv(&mut self, path : PathBuf)
    -> Result<(), &str> {
        let mut content = HashMap::<String, Vec<String>>::new();
        let f = File::open(path).unwrap();
        let reader = BufReader::new(f);
        let mut csv_reader =
            csv::Reader::from_reader(reader);
        let mut header = Vec::<String>::new();
        let mut cols = Vec::<Vec<String>>::new();
        let mut records_iter = csv_reader.records();
        if let Some(h) = records_iter.next() {
            for f_rec in h.iter() {
                if let Ok(f) = f_rec.deserialize::<String>(None) {
                    header.push(f.to_string());
                    let col = Vec::<String>::new();
                    cols.push(col);
                }
            }
        } else {
            return Err("No header in file");
        }

        for record in records_iter {
            for (i, f_rec) in record.iter().enumerate() {
                if let Ok(f) = f_rec.deserialize::<String>(None) {
                    cols[i].push(f.to_string());
                }
            }
        }

        for (title, col) in header.iter().zip(cols.iter()) {
            content.insert(title.to_string(), col.to_vec());
        }
        self.update_from_hash(content)
    }*/

    /*pub fn update_num_content_from_csv(&mut self, path : PathBuf)
    -> Result<(), &str> {
        self.tables.clear();
        let mut new_tbl = Table::new_empty(Some("A".into()));
        let mut content : HashMap<String, Vec<f64>> = HashMap::new();
        if let Some(path) = path.to_str() {
            if let Ok(txt) = csv::load_text_content(path) {
                let numcols = csv::parse_csv_unpacked(txt);
                for (name, col) in numcols {
                    if let Some(col) = col {
                        let col_dbl = col.data.as_vec().iter().map(
                            |d| *d as f64).collect();
                        new_tbl.content.insert(name, Column::Numeric(col_dbl));
                    } else {
                        println!("column {} cannot be cast to number", name);
                    }
                }
            } else {
                println!("could not load text content from path");
            }
        } else {
            println!("path not convertible to string");
        }
        self.tables.push(new_tbl);
        Ok(())
    }*/

    /// Used by both update_content_from_csv
    /// and update_content_from_queries after
    /// both those functions successfully retireve
    /// their data as HashMaps
    /*fn update_from_hash(
        &mut self,
        content : HashMap<String, Vec<String>>)
        -> Result<(), &str> {

        if content.keys().count() > 0 {
            self.used_cols = Vec::new();
            self.content = content;
            Ok(())
        } else {
            Err("Could not update data from csv.")
        }
    }*/

    /// Used to display results inside a table, for example.
    //pub fn results_as_rows(&self) -> Vec<Vec<String>> {
    //}

    /// If all the informed column names can be parsed to
    /// f64, return the parsed vectors in the same order.
    /// Return Err otherwise.
    /*pub fn cols_as_numbers(&self, cols : Vec<String>)
    -> Result<Vec<Vec<f64>>,&str> {
        let mut num_result = Vec::new();
        //num_result.push(Vec::new());
        if cols.len() == 0 {
            return Err("No columns informed.");
        }
        for c in cols {
            let mut num_col = Vec::new();
            println!("Column of interest:|{}|", c);
            if let Some(str_values) = self.content.get(&c) {
                for str_v in str_values {
                    match str_v.parse::<f64>() {
                        Ok(v) => {
                            num_col.push(v)
                        }
                        _ => {
                            return Err("Not possible to parse column.");
                        }
                    }
                }
                num_result.push(num_col);
                println!("Num result : {:?}", num_result);
            } else {
                return Err("Non-existent column");
            }
        }
        Ok(num_result)
    }*/

    /*pub fn update_content_from_queries (
        &mut self,
        queries : Vec<&PostgreQuery>)
    -> Result<(), &str> {
        let mut content = HashMap::new();
        for q in queries.iter() {
            if q.err_msg.is_none() {
                content.extend(q.results.clone());
            }
        }
        self.update_from_hash(content)
    }*/

    /*pub fn search_column(&self, name : &str) -> Option<&Column> {
        for t in self.tables.iter() {
            for c in t.content.iter() {
                if c.name == name {
                    return Some(&c);
                }
            }
        }
        None
    }*/

    /*pub fn get_subset_cols(
        &self,
        cols : Vec<String>
    ) -> Result<Vec<Vec<f64>>, &'static str> {
        let mut selected = Vec::new();
        if self.tables.len() == 0 {
            return Err("No tables loaded into the environment");
        }
        for name in cols {
            match self.search_column(&name) {
                Some(c) => {
                    if let Some(n) = c.get_if_numeric() {
                        selected.push(n);
                    } else {
                        return Err("Could not convert column to numeric");
                    }
                },
                None => {
                    println!("No column {}", name);
                    return Err("Column not found");
                }
            }
        }
        Ok(selected)
    }*/

    /*pub fn subset_cols_as_txt(&self, cols : Vec<String>) -> Result<Vec<Vec<String>>,&'static str> {
        let mut selected = Vec::new();
        if self.tables.len() == 0 {
            return Err("No tables loaded into the environment");
        }
        for name in cols {
            match self.search_column(&name) {
                Some(c) => {
                    selected.push(c.as_string_vec());
                },
                None => {
                    println!("No column {}", name);
                    return Err("Column not found");
                }
            }
        }
        Ok(selected)
    }*/

    /*pub fn col_names(&self) -> Vec<String> {
        let mut names : Vec<String> = Vec::new();
        for t in &self.tables {
            let t_col_names : Vec<String> =
                t.content.iter().map(|c| { c.name.clone() }).collect();
            names.extend(t_col_names);
        }
        names
    }*/

    // Return the names of all the column in the environment
    // that could be parsed as floating point values. This is
    // useful so Plots can query information about mappings which
    // are valid for a spatial dimension.
    /*pub fn num_col_names(&self) -> Vec<String> {
        let mut names : Vec<String> = Vec::new();
        for t in &self.tables {
            //println!("{:?}", t); println!("Numeric cols : ");
            for c in &t.content {
                if let Some(_) = c.get_if_numeric() {
                    names.push(c.name.clone());
                }
            }
        }
        names
    }*/

    /*pub fn get_table_by_name(&self, name : &str) -> Result<&Table,&'static str> {
        for t in self.tables.iter() {
            if let Some(n) = &t.name {
                if &n[..] == name {
                    return Ok(t);
                }
            }
        }
        Err("No table with the informed name")
    }*/

    pub fn last_inserted_table(&self) -> Option<Table> {
        self.tables.last().map(|t| t.clone())
    }

    fn get_table_by_index(&self, idx : usize) -> Result<&Table,&'static str> {
        match self.tables.get(idx) {
            Some(t) => Ok(t),
            None => Err("No table at informed index")
        }
    }

    pub fn get_text_at_index(&self, idx : usize) -> Option<String> {
        if let Ok(tbl) = self.get_table_by_index(idx) {
            Some(tbl.to_string())
        } else {
            None
        }
    }

    /*pub fn get_name_at_index(&self, idx : usize) -> String {
        if let Ok(tbl) = self.get_table_by_index(idx) {
            match &tbl.name {
                Some(name) => name.clone(),
                None => String::new()
            }
        } else {
            String::new()
        }
    }*/

    pub fn set_table_at_index(
        &mut self,
        content : String,
        index : usize
    ) -> Result<(), &'static str> {
        if let Ok(new_t) = Table::new_from_text(content) {
            if let Some(t) = self.tables.get_mut(index) {
                *t = new_t;
                return Ok(())
            } else {
                Err("Invalid index")
            }
        } else {
            Err("Could not parse content")
        }
    }

    pub fn all_tables_as_rows(&self) -> Vec<Vec<Vec<String>>> {
        let mut tables = Vec::new();
        for t in self.tables.iter() {
            tables.push(t.text_rows());
        }
        tables
        /*if let Some(t) = self.tables.iter().next() {
            t.as_rows()
        } else {
            Vec::new()
        }*/
    }

    pub fn all_tables(&self) -> Vec<&Table> {
        self.tables.iter().map(|t| t).collect()
    }

    pub fn all_tables_as_csv(&self) -> Vec<String> {
        let mut tbls_csv = Vec::new();
        for t in &self.tables {
            tbls_csv.push(t.to_string());
        }
        tbls_csv
    }

    pub fn append_table_from_text(
        &mut self,
        name : Option<String>,
        content : String
    ) -> Result<(), &'static str> {
        let t = Table::new_from_text(content)?;
        self.append_external_table(t)?;
        Ok(())
    }

    pub fn append_external_table(
        &mut self,
        tbl : Table
    ) -> Result<(), &'static str> {
        self.tables.push(tbl);
        Ok(())
    }

    pub fn update_from_current_source(&mut self) {
        self.tables.clear();
        match self.source {
            EnvironmentSource::Stream(ref s) => {
                if let Some(c) = s.get_last_content() {
                    self.append_table_from_text(Some("A".into()), c.clone())
                        .unwrap_or_else(|e| println!("{:?}", e));
                }
            },
            EnvironmentSource::File(ref path, ref mut content) => {
                if let Ok(mut f) = File::open(path) {
                    let mut new_content = String::new();
                    let _ = f.read_to_string(&mut new_content)
                        .unwrap_or_else(|e|{ println!("{:?}", e); 0} );
                    *content = new_content;
                } else {
                    println!("Could not re-open file");
                }
            },
            EnvironmentSource::SQLite3(_) | EnvironmentSource::PostgreSQL(_) => {
                self.send_current_query();
            },
            _ => { }
        }
    }

    pub fn clear(&mut self) {
        self.tables.clear();
    }

    // Pass this to environment source
    /*pub fn table_names_as_hash(&self) -> Option<HashMap<String, Vec<(String, String)>>> {
        let mut names = HashMap::new();
        match &self.source {
            EnvironmentSource::SQLite3(_) => {
                if let Ok(mut engine) = self.listener.engine.lock() {
                    if let Some(objs) = engine.get_table_names() {
                        for obj in objs {
                            names.insert(obj.name().into(), obj.fields().unwrap());
                        }
                    } else {
                        println!("Could not get table names from engine");
                        return None;
                    }
                } else {
                    println!("Unable to get mutable reference to engine");
                    return None;
                }
                Some(names)
            },
            _ => {
                println!("Table environment is not Sqlite3 and data could not be fetch");
                None
            }
        }
    }*/

    pub fn try_backup(&self, path : PathBuf) {
        if let Ok(engine) = self.listener.engine.lock() {
            engine.backup_if_sqlite(path);
        } else {
            println!("Unable to retrieve lock over SQL listener");
        }
    }

    pub fn last_commands(&self) -> Vec<String> {
        if let Ok(cmds) = self.listener.last_cmd.lock() {
            cmds.clone()
        } else {
            println!("Unable to acquire lock over last commands");
            Vec::new()
        }
    }

}


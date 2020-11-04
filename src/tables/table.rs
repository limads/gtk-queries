use postgres::{self, types::ToSql };
use std::convert::{TryInto};
use rust_decimal::Decimal;
use super::column::*;
use super::nullable_column::*;
//use bayes::sample::table::csv;
use bayes::sample::csv;
use std::fmt::{self, Display};
use std::string::ToString;
use num_traits::cast::ToPrimitive;
use std::str::FromStr;
use std::default::Default;

/// Data-owning structure that encapsulate named columns.
/// Implementation guarantees all columns are of the same size.
#[derive(Debug, Clone)]
pub struct Table {
    names : Vec<String>,
    cols : Vec<Column>,
    nrows : usize,
    format : TableSettings
}

impl Table {

    pub fn new(names : Vec<String>, cols : Vec<Column>) -> Result<Self, &'static str> {
        if names.len() != cols.len() {
            return Err("Differing number of names and columns");
        }
        let nrows = if let Some(col0) = cols.get(0) {
            col0.len()
        } else {
            return Err("No column zero");
        };
        for c in cols.iter().skip(1) {
            if c.len() != nrows {
                return Err("Number of rows mismatch at table creation");
            }
        }
        Ok(Self { names, cols, nrows, format : Default::default() })
    }

    pub fn new_from_text(
        source : String
    ) -> Result<Self, &'static str> {
        match csv::parse_csv_as_text_cols(&source.clone()) {
            Ok(mut cols) => {
                let mut parsed_cols = Vec::new();
                let mut names = Vec::new();
                for (name, values) in cols.drain(0..) {
                    let mut parsed_int = Vec::new();
                    let mut parsed_float = Vec::new();
                    let mut all_int = true;
                    let mut all_float = true;
                    for s in values.iter() {
                        if all_int {
                            if let Ok(int) = s.parse::<i64>() {
                                parsed_int.push(int);
                            } else {
                                all_int = false;
                            }
                        }
                        if all_float {
                            if let Ok(float) = s.parse::<f64>() {
                                parsed_float.push(float);
                            } else {
                                all_float = false;
                            }
                        }
                    }
                    match (all_int, all_float) {
                        (true, _) => parsed_cols.push(Column::I64(parsed_int)),
                        (false, true) => parsed_cols.push(Column::F64(parsed_float)),
                        _ => parsed_cols.push(Column::Str(values))
                    }
                    names.push(name);
                }
                Ok(Table::new(names, parsed_cols)?)
            },
            Err(e) => {
                println!("Error when creating table from text source : {}", e);
                Err("Could not parse CSV content")
            }
        }
    }

    pub fn flatten<'a>(&'a self) -> Result<Vec<Vec<&'a (dyn ToSql+Sync)>>, &'static str> {
        let dyn_cols : Vec<_> = self.cols.iter().map(|c| c.ref_content()).collect();
        if dyn_cols.len() == 0 {
            return Err("Query result is empty");
        }
        let n = dyn_cols[0].len();
        let mut dyn_rows = Vec::new();
        for r in 0..n {
            let mut dyn_r = Vec::new();
            for c in dyn_cols.iter() {
                dyn_r.push(c[r]);
            }
            dyn_rows.push(dyn_r);
        }
        Ok(dyn_rows)
    }

    pub fn text_rows(&self) -> Vec<Vec<String>> {
        let txt_cols : Vec<_> = self.cols.iter().map(|c| c.display_content()).collect();
        if txt_cols.len() == 0 {
            Vec::new()
        } else {
            let mut rows = Vec::new();
            let header = self.names.clone();
            rows.push(header);
            let n = txt_cols[0].len();
            for i in 0..n {
                let mut row = Vec::new();
                for c_txt in &txt_cols {
                    row.push(c_txt[i].clone());
                }
                rows.push(row);
            }
            rows
        }
    }

    /// Returns a SQL string (valid for SQlite3/PostgreSQL subset)
    /// which will contain both the table creation and data insertion
    /// commands. Binary columns are created but will hold NULL. Fails
    /// if table is not named.
    /// TODO check if SQL is valid (maybe external to the struct). SQL can
    /// be invalid if there are reserved keywords as column names.
    pub fn sql_string(&self, name : &str) -> Result<String, String> {
        if let Some(mut creation) = self.sql_table_creation(name) {
            creation += &self.sql_table_insertion(name);
            match super::sql::parse_sql(&creation[..]) {
                Ok(_) => Ok(creation),
                Err(e) => Err(format!("{}", e))
            }
        } else {
            Err(format!("Unable to form create table statement"))
        }
    }

    pub fn sql_types(&self) -> Vec<String> {
        self.cols.iter().map(|c| c.sqlite3_type().to_string()).collect()
    }

    pub fn sql_table_creation(&self, name : &str) -> Option<String> {
        let mut query = format!("CREATE TABLE {}(", name);
        for (i, (name, col)) in self.names.iter().zip(self.cols.iter()).enumerate() {
            let name = match name.chars().find(|c| *c == ' ') {
                Some(_) => String::from("\"") + &name[..] + "\"",
                None => name.clone()
            };
            query += &format!("{} {}", name, col.sqlite3_type());
            if i < self.cols.len() - 1 {
                query += ","
            } else {
                query += ")\n" //TODO ;
            }
        }
        Some(query)
    }

    /// Always successful, but query might be empty if there is no data on the columns.
    pub fn sql_table_insertion(&self, name : &str) -> String {
        let mut q = String::new();
        let mut content = self.text_rows();
        let nrows = content.len();
        if self.cols.len() <= 1 {
            return q;
        }
        content.remove(0);
        let types = self.sql_types();
        q += &format!("insert into {} values ", name)[..];
        for (line_n, line) in content.iter().enumerate() {
            q += "(";
            for (i, (f, t)) in line.iter().zip(types.iter()).enumerate() {
                match &t[..] {
                    "text" => {
                        let quoted = String::from("'") + f + "'";
                        q += &quoted
                    },
                    _ => { q +=&f }
                };
                if i < line.len() - 1 {
                    q += ","
                } else {
                    //println!("{}", nrows - 1 - line_n);
                    if line_n < nrows - 2 {
                        q += "),";
                    } else {
                        q += ");\n";
                    }
                }
            }
        }
        //println!("{}", q);
        q
    }

    /// Decide if column at ix should be displayed, according to the current display rules.
    fn show_column(&self, ix : usize) -> bool {
        if let Some(show) = self.format.show_only.as_ref() {
            show.iter()
                .find(|s| &s[..] == &self.names[ix][..] )
                .is_some()
        } else {
            true
        }
    }

    pub fn to_csv(&self) -> String {
        let mut content = String::new();
        for row in self.text_rows() {
            for (i, field) in row.iter().enumerate() {
                // Skip columns that should not be shown
                if self.show_column(i) {
                    if i >= 1 {
                        content += ",";
                    }
                    content += &field[..];
                }
            }
            content += "\n";
        }
        content
    }

    pub fn to_markdown(&self) -> String {
        let mut rows = self.text_rows();
        let mut md = String::new();
        for (i, row) in rows.drain(..).enumerate() {
            for (j, field) in row.iter().enumerate() {
                if self.show_column(j) {
                    md += &format!("|{}", field);
                }
            }
            md += &format!("|\n");
            if i == 0 {
                for j in 0..row.len() {
                    if self.show_column(j) {
                        let header_sep = match self.format.align {
                            Align::Left => "|:---",
                            Align::Center => "|:---:",
                            Align::Right => "|---:",
                        };
                        md += header_sep;
                    }
                }
                md += "|\n";
            }
        }
        md
    }

    pub fn to_html(&self) -> String {
        let mut html = String::new();
        /*
        <html>
        <body>
        <table>
        <th><td>Header</td></th>
        <tr><td>Hello</td></tr>
        </table>
        </body>
        </html>
        */
        html
    }

    pub fn shape(&self) -> (usize, usize) {
        (self.nrows, self.cols.len())
    }

    pub fn get_columns<'a>(&'a self, ixs : &[usize]) -> Columns<'a> {
        let mut cols = Columns::new();
        for ix in ixs.iter() {
            match (self.names.get(*ix), self.cols.get(*ix)) {
                (Some(name), Some(col)) => { cols = cols.take_and_push(name, col, *ix); },
                _ => println!("Column not found at index {}", ix)
            }
        }
        cols
    }

    pub fn get_column<'a>(&'a self, ix : usize) -> Option<&'a Column> {
        self.cols.get(ix)
    }

    pub fn names(&self) -> Vec<String> {
        self.names.clone()
    }

    pub fn take_columns(self) -> Vec<Column> {
        self.cols
    }

    /// If self has more rows than n, trim it. Pass self unchanged otherwise
    pub fn truncate(mut self, n : usize) -> Self {
        for col in self.cols.iter_mut() {
            col.truncate(n);
        }
        self
    }

    pub fn update_format(&mut self, settings : TableSettings) {
        self.format = settings;
    }

}

impl Display for Table {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut content = match self.format.format {
            Format::Csv => self.to_csv(),
            Format::Markdown => self.to_markdown(),
            Format::Html => unimplemented!()
        };
        write!(f, "{}", content)
    }

}

/// Referential structure that encapsulate iteration over named columns.
/// Since columns might have different tables as their source,
/// there is no guarantee columns will have the same size.
#[derive(Clone, Debug)]
pub struct Columns<'a> {
    names : Vec<&'a str>,
    cols : Vec<&'a Column>,
    ixs : Vec<usize>
}

impl<'a> Columns<'a> {

    pub fn new() -> Self {
        Self{ names : Vec::new(), cols: Vec::new(), ixs : Vec::new() }
    }

    pub fn take_and_push(mut self, name : &'a str, col : &'a Column, ix : usize) -> Self {
        self.names.push(name);
        self.cols.push(col);
        self.ixs.push(ix);
        self
    }

    pub fn take_and_extend(mut self, cols : Columns<'a>) -> Self {
        self.names.extend(cols.names);
        self.cols.extend(cols.cols);
        self.ixs.extend(cols.ixs);
        self
    }

    pub fn names(&'a self) -> &'a [&'a str] {
        &self.names[..]
    }

    pub fn indices(&'a self) -> &'a [usize] {
        &self.ixs[..]
    }

    pub fn get(&'a self, ix : usize) -> Option<&'a Column> {
        self.cols.get(ix).map(|c| *c)
    }

    // TODO move this to the implementation of try_into(.)
    /// Tries to retrieve a cloned copy from a column, performing any valid
    /// upcasts required to retrieve a f64 numeric type.
    pub fn try_numeric(&'a self, ix : usize) -> Option<Vec<f64>>
        where
            Column : TryInto<Vec<f64>,Error=&'static str>
    {
        if let Some(dbl) = self.try_access::<f64>(ix) {
            return Some(dbl);
        }
        if let Some(float) = self.try_access::<f32>(ix) {
            let cvt : Vec<f64> = float.iter().map(|f| *f as f64).collect();
            return Some(cvt);
        }
        if let Some(short) = self.try_access::<i16>(ix) {
            let cvt : Vec<f64> = short.iter().map(|s| *s as f64).collect();
            return Some(cvt);
        }
        if let Some(int) = self.try_access::<i32>(ix) {
            let cvt : Vec<f64> = int.iter().map(|i| *i as f64).collect();
            return Some(cvt);
        }
        if let Some(int) = self.try_access::<i32>(ix) {
            let cvt : Vec<f64> = int.iter().map(|i| *i as f64).collect();
            return Some(cvt);
        }
        if let Some(uint) = self.try_access::<u32>(ix) {
            let cvt : Vec<f64> = uint.iter().map(|u| *u as f64).collect();
            return Some(cvt);
        }
        if let Some(long) = self.try_access::<i64>(ix) {
            let cvt : Vec<f64> = long.iter().map(|l| *l as f64).collect();
            return Some(cvt);
        }
        if let Some(dec) = self.try_access::<Decimal>(ix) {
            let mut cvt : Vec<f64> = Vec::new();
            for d in dec.iter() {
                if let Some(f) = d.to_f64() {
                    cvt.push(f);
                } else {
                    println!("Invalid decimal conversion");
                    return None;
                }
            }
            return Some(cvt);
        }
        println!("Invalid column conversion");
        None
    }

    pub fn try_access<T>(&'a self, ix : usize) -> Option<Vec<T>>
        where
            Column : TryInto<Vec<T>, Error=&'static str>
    {
        if let Some(c) = self.get(ix) {
            let v : Result<Vec<T>,_> = c.clone().try_into();
            match v {
                Ok(c) => { Some(c) },
                Err(_) => { /*println!("{}", e);*/ None }
            }
        } else {
            println!("Invalid column index");
            None
        }
    }

}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Csv,
    Markdown,
    Html
}

impl FromStr for Format {

    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "CSV" => Ok(Format::Csv),
            "HTML" => Ok(Format::Html),
            "Markdown" => Ok(Format::Markdown),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone)]
pub enum Align {
    Left,
    Center,
    Right
}

#[derive(Debug, Clone)]
pub enum BoolField {
    Char,
    CharUpper,
    Word,
    WordUpper,
    Integer
}

impl FromStr for BoolField {

    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "'t' or 'f'" => Ok(Self::Char),
            "'T' or 'F'" => Ok(Self::CharUpper),
            "'true' or 'False'" => Ok(Self::Word),
            "'TRUE' or 'FALSE'" => Ok(Self::WordUpper),
            "'1' or '0'" => Ok(Self::WordUpper),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone)]
pub enum NullField {
    Word,
    WordUpper,
    Omit
}

impl FromStr for NullField {

    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "null" => Ok(Self::Word),
            "NULL" => Ok(Self::WordUpper),
            "Omit'" => Ok(Self::Omit),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableSettings {
    pub format : Format,
    pub align : Align,
    pub bool_field : BoolField,
    pub null_field : NullField,
    pub prec : usize,
    pub show_only : Option<Vec<String>>
}

impl Default for TableSettings {

    fn default() -> Self {
        Self {
            format : Format::Csv,
            align : Align::Left,
            bool_field : BoolField::Word,
            null_field : NullField::Omit,
            prec : 8,
            show_only : None
        }
    }

}

pub fn full_csv_display(tbl : &mut Table, cols : Vec<String>) -> String {
    let show = if cols.len() == 0 {
        None
    } else {
        Some(cols)
    };
    let fmt = TableSettings {
        format : Format::Csv,
        align : Align::Left,
        bool_field : BoolField::Char,
        null_field : NullField::WordUpper,
        prec : 12,
        show_only : show
    };
    tbl.update_format(fmt);
    format!("{}", tbl)
}



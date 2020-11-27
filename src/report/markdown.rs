use std::cmp::PartialEq;
use pulldown_cmark::{self, Tag, CodeBlockKind, Event};
use std::str::FromStr;
use anyhow::{self, Result, Error};
use std::path::PathBuf;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::tokenizer::{Tokenizer, Token};
use sqlparser::dialect::keywords::Keyword;
use queries::tables::table::Table;
use queries::tables;

pub mod client;

/// SQL statement that should be bound to local variables at 'into'
/// and/or uses template variables at 'using'. The statement is built
/// using the PL/pgSQL-like syntax:
/// execute 'select * from location where id = $1;' into locations using loc_id;
#[derive(Debug, Clone)]
pub struct BoundSQL {
    into : Vec<String>,
    using : Vec<String>,
    stmt : String
}

/// Collect the names until either a final non-name keyword is found (returning it) or there
/// are no more tokens to parse.
fn get_name_list(tk_iter : &mut std::vec::Drain<'_, Token>) -> Result<(Vec<String>, Option<Token>), String> {
    let mut names = Vec::new();
    let mut separated = true;
    loop {
        match tk_iter.next() {
            Some(Token::Word(w)) => {
                match w.keyword {
                    Keyword::NoKeyword => {
                        if separated {
                            names.push(w.value.to_string());
                            separated=false;
                        } else {
                            return Err(format!("Non-separated name {}", w.value));
                        }
                    },
                    k => { return Ok((names, Some(Token::Word(w)))); }
                }
            },
            Some(Token::Whitespace(_)) => { },
            Some(Token::Comma) => {
                if !separated {
                    separated=true;
                } else {
                    return Err(format!("Invalid statement (double comma)"));
                }
            },
            Some(Token::SemiColon) => { return Ok((names, Some(Token::SemiColon))) },
            Some(token) => { return Err(format!("Invalid end token: {}", token)); },
            None => { return Ok((names, None)); }
        }
    }
}

fn parse_into_clause(
    tk_iter : &mut std::vec::Drain<'_, Token>,
    stmt : String
) -> Result<BoundSQL, anyhow::Error> {
    if let Ok((into_list, end_into_tk)) = get_name_list(tk_iter) {
        match end_into_tk {
            Some(Token::Word(w)) => match w.keyword {
                Keyword::USING => {
                    parse_using_clause(tk_iter, into_list, stmt)
                },
                k => Err(Error::msg(format!("Invalid end keyword: {:?}", k)))
            },
            Some(Token::SemiColon) => {
                Ok(BoundSQL{ into : into_list, using : Vec::new(), stmt })
            },
            _ => Err(Error::msg(format!("Invalid end token: {:?}", end_into_tk)))
        }
    } else {
        Err(Error::msg(format!("Invalid statement")))
    }
}

fn parse_using_clause(
    tk_iter : &mut std::vec::Drain<'_, Token>,
    into_list : Vec<String>,
    stmt : String
) -> Result<BoundSQL, anyhow::Error> {
    if let Ok((using_list, end_using_k)) = get_name_list(tk_iter) {
        if end_using_k == Some(Token::SemiColon) || end_using_k == None {
            Ok(BoundSQL{ into : into_list, using : using_list, stmt })
        } else {
            Err(Error::msg(format!("Invalid end of input")))
        }
    } else {
        Err(Error::msg("Using keyword does not have valid names"))
    }
}

impl FromStr for BoundSQL {

    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dialect = PostgreSqlDialect {};
        let mut tokenizer = Tokenizer::new(&dialect, s);
        let mut tokens = tokenizer.tokenize().unwrap();
        let mut tk_iter = tokens.drain(..);
        if let Some(tk) = tk_iter.next() {
            match tk {
                Token::Word(w) => if w.keyword == Keyword::EXECUTE {
                    match tk_iter.nth(1) {
                        Some(Token::SingleQuotedString(s)) => {
                            let stmt = s.to_string();
                            match tk_iter.nth(1) {
                                Some(Token::Word(w)) => if w.keyword == Keyword::INTO {
                                    parse_into_clause(&mut tk_iter, stmt)
                                } else {
                                    if w.keyword == Keyword::USING {
                                        parse_using_clause(&mut tk_iter, Vec::new(), stmt)
                                    } else {
                                        Err(Error::msg("Statement missing INTO and/or USING clause"))
                                    }
                                },
                                _ => {
                                    Err(Error::msg("Statement missing INTO and/or USING clause"))
                                }
                            }
                        },
                        _ => {
                            Err(Error::msg("Missing SQL statement"))
                        }
                    }
                } else {
                    Err(Error::msg("First keyword should be EXECUTE"))
                },
                token => Err(Error::msg(format!("Invalid start token: {:?}", token)))
            }
        } else {
            Err(Error::msg("No EXECUTE keyword"))
        }
    }

}

/// There are a few ways in which we can implement code cells for Rust code:
///
/// 1. Make them simply a series of code blocks pasted one after the other in a main() call
/// 2. Make them a series of local anonymous (or non-anonymous) code blocks inside { } in a main() call
/// 3. Make them a series of nested calls for the signature Fn()->Option<Box<dyn Display>>.
///
/// # Option 1
/// Option 1 requires that output is captured using an expansion for the last expression for each block (e.g.):
/// ```rust
/// let b = 1+1
/// b
/// ```
/// The last line would be expanded into
/// results.push(format!("{}", b));
/// and the final output is captured by serializing the results vector into stdout of the executable (e.g. JSON array).
/// We assume the user always return T : Display, allowing compilation to fail otherwise. Only full-document execution is
/// possible, and all cells can mutate any name mutably bound.
///
/// # Option 2
/// Option 2 gives the highest locality to the code cells: all cells are then effectively separate programs, that coordinate
/// only by changing an external data source (such as a database or file). Assuming this external data source is not changed
/// between any pair of calls allow interesting parallelism optimizations. We could check that, for example, by requiring that all
/// SQL interaction happen from within SQL cells, and checking those cells do not make insert/update/delete between a pair of code blocks.
///
/// We could call cells independently from one another.
/// we implement it like so:
/// ```rust
/// results.push(format!("{}", { let b = 1+1; b }));
/// ```
/// Where the user-supplied code is what is inside the block. This is easier to implement than (1) since we do not
/// need to parse the last expression away from the rest of the block,
/// only fit it into the format! call.
///
/// We could also adopt a strategy where the user could name each code block:
/// results.push(format!("{}", 'my_code : { let b = 1+1; b }));
/// and then we could use syn to parse each named code block and build and save the resulting computation
/// from each block into a HashMap<String, String> where the keys would be the named blocks. We would start
/// the executable by passing the parsed names as arguments to build the keys. Then, only the return values
/// for each block would be available for the user, if he referred to each block by name at this hashmap (or read
/// if from some environment variable such as env::var('CODE'). Alternatively, he could refer by cell order
/// by calling from an array of environment variables such as env::var('$CELL[0]'). This of course would preclude
/// any possible parallelism optimizations.
///
/// We can expand the SQL execute query blocks in a global rather than local scope, so any variables bound from SQL queries
/// would be available to all cells below the point where the query was made. manipulation statements could then
/// change the global database state, which could be read by the cells below. But to bind names from Rust to SQL,
/// we would require that all names should be located on the same block, so the expansion would happen inside the block.
/// we would then have: global queries independent of program state OR local statements dependent on program state.
///
/// # Option 3
/// Option 3 allows the names of previous cells to be bound by reference to cells below them. The limitations of Option 2
/// are now lifted, since expanding the queries even inside the blocks would continue to make the variable bindings available.
/// We can mix strategies as well: Leave as nested closures everything that has variable dependencies; and leave as independent
/// serial closures everything that do not have variable dependencies. The resulting closure vector can then always be parallelized,
/// since every vector element is an independent execution path of the asynchronous execution DAG.
#[derive(Clone, Debug)]
enum CellKind {

    /// Plain SQL statement, to be rendered as a table
    SQL,

    /// SQL statement bound to or using local variables
    Binding(BoundSQL),

    /// User-supplied Rust code
    Rust,

    /// Any markdown outside code block
    Markdown,

    /// Link to XML plot definition, to be rendered as SVG
    Plot
}

#[derive(Debug)]
struct Cell {
    content : String,
    kind : CellKind
}

#[derive(Debug)]
struct Notebook {
    cells : Vec<Cell>,
    style : Option<PathBuf>
}

impl Notebook {

    /// Generates the full rendered document.
    pub fn weave(&self) -> Result<String, anyhow::Error> {
        unimplemented!()
    }

    fn expand_binding(binding : &BoundSQL, cell_ix : usize) -> Result<String, anyhow::Error> {
        // binding.into
        // binding.using
        // binding.stmt
        unimplemented!()
    }

    fn cell_start_tag(ix : usize) -> String {
        format!("println!(\"<cell index={}>\");", ix)
    }

    fn cell_end_tag() -> &'static str {
        "println!(\"</cell>\")"
    }

    fn expand_sql(sql : &str, cell_ix : usize) -> Result<String, anyhow::Error> {
        let stmts = sqlparser::parser::ParseSql(PostgreSqlDialect)?;
        let mut exp = String::new();
        for stmt in stmts {
            match stmt {
                Statement::Query(_) => {
                    exp += &Self::cell_start_tag(cell_ix);
                    exp += &format!("queries::report::client::query(&mut cli, \"{}\").map(|tbl| tbl.to_markdown() )?;", stmt);
                    exp += Self::cell_end_tag();
                },
                _ => {
                    exp += &format!("queries::report::client::exec(&mut cli, \"{}\")?;", stmt);
                }
            }
        }
        Ok(exp)
    }

    fn generate_source(&self, conn : &str) -> Result<String, anyhow::Error> {
        let mut src = String::from("extern crate postgres; extern crate queries; fn main() -> Result<String, String> {");
        src += &format!("let mut cli = Client::connect(\"{}\").map_err(|e| format!(\"{}\", e)?;");
        for (ix, cell) in &self.cells {
            match cell.kind {
                CellKind::SQL => {
                    src += Self::expand_sql(&cell.content[..], ix)?;
                },
                CellKind::BoundSQL(b) => {
                    src += Self::expand_binding(&b, ix)?;
                }
            }
        }
        src += println!("}");
        Ok(src)
    }

    /// Generates an executable based on the current cells.
    pub fn tangle(&self, path : &str) -> Result<(), anyhow::Error> {

    }

}

impl FromStr for Notebook {

    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut cells = Vec::new();
        let parser = pulldown_cmark::Parser::new(s);
        let mut cell : Option<Cell> = None;
        for (event, offset) in parser.into_offset_iter() {
            println!("{:?}", event);
            match event {
                Event::Start(tag) => {
                    let mut cell = match tag {
                        Tag::CodeBlock(CodeBlockKind::Fenced(lang)) => {
                            let kind = match lang.as_ref() {
                                "sql" => {
                                    let binding : Result<BoundSQL, _> = (&s[offset.clone()]).parse();
                                    if let Ok(b) = binding {
                                        CellKind::Binding(b)
                                    } else {
                                        CellKind::SQL
                                    }
                                },
                                "rust" => CellKind::Rust,
                                other => {
                                    let invalid_msg = format!("Invalid code block: {}", other);
                                    return Err(anyhow::Error::msg(invalid_msg));
                                }
                            };
                            Cell{ content : String::new(), kind }
                        },
                        _ => Cell{ content : String::new(), kind: CellKind::Markdown }
                    };
                    cell.content += &s[offset];
                    cells.push(cell);
                }
                _ => { }
            }
        }
        Ok(Self{ cells, style : None})
    }

}

#[test]
fn parse_md() {
    let txt = r#"# Header 1
## Header 2

Paragraph

```sql
select * from patients;
```

```rust
let a = 2;
```

```rust
let b = 3;
```

```rust
let d = a + b;
println!("{}", a);
```
"#;
    let doc : Notebook = txt.parse().unwrap();
    println!("{:?}", doc);
}

// Each variable in the using clause should be an impl Iterator<Item=T : ToSql>
// The variable named at the into clause will be a Vec<Row>
#[test]
fn parse_execute() {
    let cmd = "EXECUTE 'SELECT count(*) FROM mytable WHERE inserted_by = $1 AND inserted <= $2' INTO c USING checked_user, checked_date;";
    let bound : BoundSQL = cmd.parse().unwrap();
    println!("{:?}", bound);
}



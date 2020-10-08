use std::cmp::PartialEq;
use pulldown_cmark::{self, Tag, CodeBlockKind, Event};
use std::str::FromStr;
use anyhow::{self, Result, Error};
use std::path::PathBuf;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::tokenizer::{Tokenizer, Token};
use sqlparser::dialect::keywords::Keyword;

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



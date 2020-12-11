use std::io;
use std::fs::File;
use std::path::Path;
use libxml::tree::document::{Document, SaveOptions};
use libxml::parser::Parser;
use libxml::tree::node::Node;
use std::io::{Read, Write};
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::environment::TableEnvironment;
use std::default::Default;
use xml::reader::{EventReader, XmlEvent};
use std::io::BufReader;
use std::mem;
use crate::tables::table::TableSettings;

fn search_data(t_env : &TableEnvironment, col_name : &str) -> Option<Vec<String>> {
    println!("Looking for column name {}", col_name);
    for tbl in t_env.all_tables().iter() {
        if let Some(ix) = tbl.names().iter().position(|name| &name[..] == &col_name[..] ) {
            let mut tbl = tbl.clone();
            let mut format : TableSettings = Default::default();
            format.prec = 4;
            tbl.update_format(format);
            let col_data = tbl.text_cols().remove(ix);
            println!("Found column data: {:?}", col_data);
            return Some(col_data);
        }
    }
    println!("Column name {} not found", col_name);
    None
}

/*
Libreoffice convention:
<table>
<table-column />
<table-column />
<table-row>
    <table-cell>
        <p>
            <span>
                <placeholder>content</placeholder>
            </span>
        </p>
    </table-cell>
</table-row>
*/

fn expand_rows(doc : &Document, row_grandparent : Node, n : usize) -> Result<(), String> {
    let ns = row_grandparent.get_namespace();
    let props = row_grandparent.get_properties();
    let mut tbl = row_grandparent.get_parent().ok_or(format!("Row grandparent does not have table parent"))?;
    mem::drop(row_grandparent);
    
    for i in 0..n {
        let mut new_row = Node::new("table-row", ns.clone(), doc)
            .map_err(|e| format!("{:?}", e))?;
        for (k, v) in &props {
            new_row.set_property(&k, &v);
        }
        tbl.add_child(&mut new_row).map_err(|e| format!("Error adding new table row: {}", e))?;
    }
    Ok(())
}

fn access_rows(row_grandparent : &Node) -> Result<Vec<Node>, String> {
    println!("Row grandparent name: {}", row_grandparent.get_name());
    let tbl = row_grandparent.get_parent()
        .ok_or(format!("Missing table parent node"))
        .and_then(|node| { 
            if node.get_name() == "table" {
                Ok(node)
            } else {
                Err(format!("Parent of row is not a table, but a {}", node.get_name()))
            }
        })?;
    println!("Table children: {:?}", tbl.get_child_nodes().iter().map(|c| c.get_name()).collect::<Vec<_>>());
    let row_nodes : Vec<Node> = tbl.get_child_nodes()
        .iter()
        .cloned()
        .filter(|node| &node.get_name()[..] == "table-row")
        .collect();
    if row_nodes.len() == 0 {
        return Err(format!("Should have found at least one placeholder row node"));
    }
    println!("Found {} row nodes", row_nodes.len());
    Ok(row_nodes)
}

fn guarantee_table_length(doc : &Document, mut row_grandparent : Node, data_length : usize) -> Result<(), String> {
    let row_len = {
        let rows = access_rows(&row_grandparent)?;
        rows.len()
    };
    println!("Required data length: {}", data_length);
    println!("Actual length: {}", row_len);
    if row_len-1 < data_length {
        println!("Table needs expanding");
        expand_rows(&doc, row_grandparent.clone(), data_length)?; // TODO verify that this clone is valid
    }
    let new_len = access_rows(&row_grandparent)?.len();
    println!("New length: {}", new_len);
    assert!(new_len - 1 == data_length);
    Ok(())
}

/*fn create_cell_paragraph(
    doc : &Document, 
    // row : &mut Node, 
    row_grandparent : &Node, 
    row_ix : usize, 
    col_ix : usize
) -> Result<Node, String> {    
    
    Ok(new_cell)
}*/

fn expand_column(
    doc : &Document,
    old_node : Node,
    mut row_grandparent : Node, 
    t_env : &TableEnvironment,
    col_ix : usize, 
    col_name : &str
) -> Result<(), String> {
    let text_ns = old_node.get_namespace();
    let mut text_props = old_node.get_properties();
    let col_data = search_data(&t_env, col_name).ok_or(format!("Missing column {}", col_name))?;
    if col_data.len() == 0 {
        return Err(format!("Empty table for {}", col_name));
    }
    /*let col_data = search_data(&t_env, col_name).ok_or(format!("Missing column {}", col_name))?;
    if col_data.len() == 0 {
        return Err(format!("Empty table for {}", col_name));
    }*/
    let (mut row, mut col) = (0, 0);
    let ns = row_grandparent.get_namespace();
    guarantee_table_length(&doc, row_grandparent.clone(), col_data.len())?;
    
    let mut all_rows = access_rows(&row_grandparent)?;
    println!("All rows: {:?}", all_rows.len());
    mem::forget(row_grandparent);
    
    {
        let mut par_parent = old_node.get_parent().ok_or(format!("No paragraph parent available"))?;
        let mut span = Node::new("span", par_parent.get_namespace(), doc)
            .map_err(|e| format!("{:?}", e))?;
        let content = format!("{}", col_name);
        span.set_content(&content[..]).map_err(|e| format!("{}", e))?;
        par_parent.replace_child_node(span, old_node).map_err(|e| format!("{}", e))?;
        text_props = par_parent.get_properties();
    }

    println!("Data length: {}", col_data.len());
    println!("Row length: {}", all_rows.len());
    
    let mut n_rows = 0;
    
    // Must skip the first header row here, because it cannot be mutably referenced and is not necessary.
    for (row_ix, (data, mut row)) in col_data.iter().zip(all_rows.drain(0..).skip(1)).enumerate() {
        // let mut cell = create_cell_paragraph(&doc, &mut row, &row_grandparent, row_ix, col_ix)?;
        let content = format!("{}", col_data.get(row_ix).ok_or(format!("Missing data for row {}", row_ix))?);
        let mut span = Node::new("span", text_ns.clone(), doc)
            .map_err(|e| format!("{:?}", e))?;
        span.set_content(&content[..]).map_err(|e| format!("{}", e))?;
        let mut new_cell = Node::new("table-cell", ns.clone(), doc)
            .map_err(|e| format!("{:?}", e))?;
        let mut new_par = Node::new("p", text_ns.clone(), doc)
            .map_err(|e| format!("{:?}", e))?;
        for (k, v) in &text_props {
            span.set_property(&k, &v);
            new_par.set_property(&k, &v);
        }
        new_par.add_child(&mut span).map_err(|e| format!("{}", e))?;
        new_cell.add_child(&mut new_par);
        row.add_child(&mut new_cell).map_err(|e| format!("{}", e))?;
        n_rows += 1;
    }
    
    // Check we iterated exactly for the same number of rows as the data vector lenght.
    assert!(n_rows == col_data.len());
    
    Ok(())
}

fn expand_paragraph(
    doc : &Document,
    mut old_node : Node,
    mut par_parent : Node, 
    t_env : &TableEnvironment, 
    col_name : &str
) -> Result<(), String> {
    let col_data = search_data(&t_env, col_name)
        .ok_or(format!("Missing column {}", col_name))?;
    if col_data.len() == 0 {
        return Err(format!("Empty table for {}", col_name));
    }
    if col_data.len() > 1 {
        return Err(format!("Multi-line output at text placeholder {}", col_name));
    }
    // println!("Should set: {}", format!("<text:span>{:?}</text:span>", col_data.get(0)));
    let mut span = Node::new("span", par_parent.get_namespace(), doc)
        .map_err(|e| format!("{:?}", e))?;
    let content = format!("{}", col_data.get(0).ok_or(format!("Missing data for row 0"))?);
    span.set_content(&content[..]).map_err(|e| format!("{}", e))?;
    par_parent.replace_child_node(span, old_node).map_err(|e| format!("{}", e))?;
    println!("New content: {}", par_parent.get_content());
    Ok(())
}

/// Checks if the placeholder is inside a table. If it is, return the row grandparent (or grand-grandparent
/// depending on how deep the placeholder is nested). If it is not, return the placeholder parent (usually
/// a span or paragraph). Writes if it is or is not a table placeholder at the inside_table variable.
fn determine_table_placeholder(tag : &Node, inside_table : &mut bool) -> Result<Node, String> {
    if let Some(parent) = tag.get_parent() {
        if let Some(grandparent) = parent.get_parent() {
            // It might be the case that the placeholder is nested in a span->p or just a p.
            if grandparent.get_name() == "table-cell" {
                *inside_table = true;
                let row_grandparent = grandparent
                    .get_parent()
                    .ok_or(format!("Missing row grandparent"))?;
                if row_grandparent.get_name() != "table-row" {
                    return Err(format!("Expected row grandparent, found {}", row_grandparent.get_name()));
                }
                Ok(row_grandparent)
            } else {
                if let Some(grand_grandparent) = grandparent.get_parent() {
                    if grand_grandparent.get_name() == "table-cell" {
                        *inside_table = true;
                        let row_grandparent = grand_grandparent
                            .get_parent()
                            .ok_or(format!("Missing row grand-grandparent"))?;
                        if row_grandparent.get_name() != "table-row" {
                            return Err(format!("Expected row grandparent, found {}", row_grandparent.get_name()));
                        }
                        Ok(row_grandparent)
                    } else {
                        *inside_table = false;
                        Ok(parent)
                    }
                } else {
                    *inside_table = false;
                    Ok(parent)
                }
            }
        } else {
            Err(format!("Missing placeholder grandparent"))
        }
    } else {
        Err(format!("Missing placeholder parent"))
    }
}

pub fn write_report(
    template_path : &str, 
    out_path : &str, 
    table_env : Rc<RefCell<TableEnvironment>>
) -> Result<(), String> {
    if template_path == out_path {
        return Err(format!("Template and output path cannot be the same"));
    }
    let t_env = table_env.try_borrow().map_err(|_| format!("Could not borrow table environment"))?;
    let mut content = String::new();
    let mut src = File::open(template_path).map_err(|e| format!("{}", e) )?;
    src.read_to_string(&mut content);
    let parser : Parser = Default::default();
    let mut doc = parser.parse_string(&content)
        .expect("Failed parsing XML file");
    let root = doc.get_root_element()
        .expect("Root node not found");
    let mut tags = root
        .findnodes("//*")
        .expect("No nodes");
    mem::drop(root);
    
    /*let parser = EventReader::new(content.as_bytes());
    for e in parser {
        // Start element has name, attributes and namespace fields; end element has name field.
        // Ok(StartElement({urn:oasis:names:tc:opendocument:xmlns:text:1.0}text:placeholder
        // Ok(Characters(<n_used_refraction>))
        // Ok(EndElement({urn:oasis:names:tc:opendocument:xmlns:text:1.0}text:placeholder))
        println!("{:?}", e);
    }*/
    
    // Placeholders are searched for in the order they appear in tags.iter(). Therefore,
    // we might find a sequence inside a table among all the placeholders. This variable
    // keeps track what column of the table we are expanding over (if any).
    let mut inside_table = false; 
    let mut expanding_column : Option<usize> = None;
    
    for (i, tag) in tags.drain(0..).enumerate() {
        if tag.get_name() == "placeholder" {
            let col_name_string = &tag.get_content();
            let col_name = col_name_string[1..(col_name_string.len() - 1)].to_string();
            let parent = determine_table_placeholder(&tag, &mut inside_table)?;
            println!("({}) Inside table = {:?}", col_name, inside_table);
            println!("({}) Expanding column = {:?}", col_name, expanding_column);
            match inside_table {
                true => match expanding_column {
                    Some(ref mut ix) => {
                        *ix += 1;
                        expand_column(&doc, tag, parent, &t_env, *ix, &col_name)?;
                    },
                    None => {
                        let ix = 0;
                        expand_column(&doc, tag, parent, &t_env, ix, &col_name)?;
                        expanding_column = Some(ix);
                    }
                }, 
                false => {
                    // println!("Found placeholder outside table: {}", tag.get_content());
                    // mem::drop(tag);
                    expand_paragraph(&doc, tag, parent, &t_env, &col_name)?;
                    expanding_column = None;
                }
            }
        }
    }
    
    let mut dst = File::create(out_path)
        .map_err(|e| format!("{}", e))?;
    let mut options : SaveOptions = Default::default();
    options.format = true;
    options.non_significant_whitespace = true;
    dst.write_all(doc.to_string_with_options(options).as_bytes())
        .map_err(|e| format!("{}", e))?;
    Ok(())
}



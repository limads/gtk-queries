use libxml::tree::node::Node;
use gdk::RGBA;
use cairo::Context;
use super::super::context_mapper::ContextMapper;
use std::collections::HashMap;
// use std::f64::consts::PI;
use super::utils;
// use super::super::context_mapper::Coord2D;
// use cairo::ScaledFont;
use super::text::{FontData, draw_label};
use super::*;

#[derive(Debug)]
pub struct TextMapping {
    x : Vec<f64>,
    y : Vec<f64>,
    text : Vec<String>,
    font : FontData,
    color : RGBA,
    col_names : [String; 3]
}

impl TextMapping {

    pub fn new(node : &Node) -> Self {
        let x = Vec::<f64>::new();
        let y = Vec::<f64>::new();
        let text = Vec::<String>::new();
        let color = gdk::RGBA{
            red:0.0,
            green:0.0,
            blue:0.0,
            alpha : 0.0
        };
        let font = FontData::create_standard_font();
        let col_names = [
            String::from("None"),
            String::from("None"),
            String::from("None")
        ];
        let mut mapping = TextMapping{ x, y, text, font, color, col_names};
        mapping.update_layout(node);
        mapping
    }

    // Calls to update_data(.) can call this
    // function to update the text before the
    // call to draw(.).
    pub fn set_text_data(&mut self, text : &Vec<String>) {
        self.text = text.clone();
    }
}

impl Mapping for TextMapping {

    fn draw(&self, mapper : &ContextMapper, ctx : &Context) {
        ctx.save();
        if !((self.x.len() == self.y.len()) && (self.x.len() == self.text.len())) {
            println!("Invalid dimensions at textual mapping");
            println!("x: {}; y: {}; t: {}", self.x.len(), self.y.len(), self.text.len());
        }
        ctx.set_source_rgb(
            self.color.red,
            self.color.green,
            self.color.blue
        );
        self.font.set_font_into_context(&ctx);
        for ((x, y), t) in self.x.iter().zip(self.y.iter()).zip(self.text.iter()) {
            if mapper.check_bounds(*x, *y) {
                let pos = mapper.map(*x, *y);
                draw_label(
                    &self.font.sf,
                    ctx,
                    t,
                    pos,
                    false,
                    (true, true),
                    None,
                    None
                );
            } else {
                //println!("Out of bounds mapping");
            }
        }
        ctx.restore();
    }

    fn update_data(&mut self, values : Vec<Vec<f64>>) {
        self.x = values[0].clone();
        self.y = values[1].clone();
    }

    fn update_extra_data(&mut self, values : Vec<Vec<String>>) {
        if let Some(tv) = values.get(0) {
            self.text = tv.clone();
        } else {
            println!("Text data vector is empty");
        }
    }

    fn update_layout(&mut self, node : &Node) {
        let props = utils::children_as_hash(node, "property");
        self.color = props["color"].parse().unwrap();
        self.font = FontData::new_from_string(&props["font"]);
        self.col_names[0] = props["x"].clone();
        self.col_names[1] = props["y"].clone();
        self.col_names[2] = props["text"].clone();
    }

    fn properties(&self) -> HashMap<String, String> {
        let mut properties = MappingType::Text.default_hash();
        if let Some(e) = properties.get_mut("color") {
            *e = self.color.to_string();
        }
        if let Some(e) = properties.get_mut("font") {
            *e = self.font.description();
        }
        if let Some(e) = properties.get_mut("x") {
            *e = self.col_names[0].clone();
        }
        if let Some(e) = properties.get_mut("y") {
            *e = self.col_names[1].clone();
        }
        if let Some(e) = properties.get_mut("text") {
            *e = self.col_names[2].clone();
        }
        properties
    }

    fn mapping_type(&self) -> String {
        "text".into()
    }

    fn get_col_name(&self, col : &str) -> String {
        match col {
            "x" => self.col_names[0].clone(),
            "y" => self.col_names[1].clone(),
            "text" => self.col_names[2].clone(),
            _ => String::new()
        }
    }

    fn get_ordered_col_names(&self) -> Vec<(String,String)> {
        vec![
            (String::from("x"), self.get_col_name("x")),
            (String::from("y"), self.get_col_name("y")),
            (String::from("text"), self.get_col_name("text"))
        ]
    }

    fn set_col_name(&mut self, col : &str, name : &str) {
        match col {
            "x" => { self.col_names[0] = name.into(); },
            "y" => { self.col_names[1] = name.into(); },
            "text" => { self.col_names[2] = name.into(); },
            _ => { }
        }
    }

    fn get_hash_col_names(&self) -> HashMap<String, String> {
        let mut cols = HashMap::new();
        cols.insert("x".into(), self.col_names[0].clone());
        cols.insert("y".into(), self.col_names[1].clone());
        cols.insert("text".into(), self.col_names[2].clone());
        cols
    }

    fn set_col_names(&mut self, cols : Vec<String>) -> Result<(), &'static str> {
        if cols.len() != 3 {
            Err("Wrong number of columns.")
        } else {
            self.set_col_name("x", &cols[0]);
            self.set_col_name("y", &cols[1]);
            self.set_col_name("text", &cols[2]);
            Ok(())
        }
    }
}


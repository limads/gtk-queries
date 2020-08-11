use libxml::tree::node::Node;
use gdk::RGBA;
use cairo::Context;
use super::super::context_mapper::ContextMapper;
use std::collections::HashMap;
use std::f64::consts::PI;
use super::utils;
// use super::super::context_mapper::Coord2D;
// use cairo::ScaledFont;
// use super::super::text::{FontData, draw_label};
use super::*;

#[derive(Debug)]
pub struct ScatterMapping {
    color : RGBA,
    x : Vec<f64>,
    y : Vec<f64>,
    radius : f64,
    col_names : [String; 2],
    source : String
}

impl ScatterMapping {
    pub fn new(node : &Node) -> Self {
        let color = gdk::RGBA{
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha : 0.0
        };
        let radius = 1.0;
        let x = Vec::<f64>::new();
        let y = Vec::<f64>::new();
        let col_names = [
            String::from("None"),
            String::from("None")
        ];
        let source = String::new();
        let mut mapping = ScatterMapping{color, x, y, radius, col_names, source};
        mapping.update_layout(node);
        mapping
    }
}

impl Mapping for ScatterMapping {

    fn draw(&self, mapper : &ContextMapper, ctx : &Context) {
        ctx.save();
        ctx.set_source_rgb(self.color.red, self.color.green, self.color.blue);
        for (x, y) in self.x.iter().zip(self.y.iter()) {
            if mapper.check_bounds(*x, *y) {
                let pos = mapper.map(*x, *y);
                ctx.arc(pos.x, pos.y, self.radius, 0.0, 2.0*PI);
                ctx.fill();
                ctx.stroke();
            } else {
                //println!("Out of bounds mapping");
            }
        }
        ctx.restore();
    }

    //fn new(&self, HashMap<String, String> properties);
    fn update_data(&mut self, values : Vec<Vec<f64>>) {
        self.x = values[0].clone();
        self.y = values[1].clone();
    }

    fn update_extra_data(&mut self, _values : Vec<Vec<String>>) {
        println!("Mapping has no extra data");
    }

    fn update_layout(&mut self, node : &Node) {
        let props = utils::children_as_hash(node, "property");
        self.color = props["color"].parse().unwrap();
        self.radius = props["radius"].parse().unwrap();
        self.col_names[0] = props["x"].clone();
        self.col_names[1] = props["y"].clone();
    }

    fn properties(&self) -> HashMap<String, String> {
        let mut properties = MappingType::Scatter.default_hash();
        if let Some(e) = properties.get_mut("color") {
            *e = self.color.to_string();
        }
        if let Some(e) = properties.get_mut("radius") {
            *e = self.radius.to_string();
        }
        if let Some(e) = properties.get_mut("x") {
            *e = self.col_names[0].clone();
        }
        if let Some(e) = properties.get_mut("y") {
            *e = self.col_names[1].clone();
        }
        properties
    }

    fn mapping_type(&self) -> String {
        "scatter".into()
    }

    fn get_col_name(&self, col : &str) -> String {
        match col {
            "x" => self.col_names[0].clone(),
            "y" => self.col_names[1].clone(),
            _ => String::new()
        }
    }

    fn get_ordered_col_names(&self) -> Vec<(String,String)> {
        vec![
            (String::from("x"), self.get_col_name("x")),
            (String::from("y"), self.get_col_name("y"))
        ]
    }

    fn get_hash_col_names(&self) -> HashMap<String, String> {
        let mut cols = HashMap::new();
        cols.insert("x".into(), self.col_names[0].clone());
        cols.insert("y".into(), self.col_names[1].clone());
        cols
    }

    fn set_col_name(&mut self, col : &str, name : &str) {
        match col {
            "x" => { self.col_names[0] = name.into(); },
            "y" => { self.col_names[1] = name.into(); },
            _ => { }
        }
    }

    fn set_col_names(&mut self, cols : Vec<String>) -> Result<(), &'static str> {
        if cols.len() != 2 {
            Err("Wrong number of columns.")
        } else {
            self.set_col_name("x", &cols[0]);
            self.set_col_name("y", &cols[1]);
            Ok(())
        }
    }

    fn set_source(&mut self, source : String) {
        self.source = source;
    }

    fn get_source(&self) -> String {
        self.source.clone()
    }

}


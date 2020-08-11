use libxml::tree::node::Node;
use gdk::RGBA;
use cairo::Context;
use super::super::context_mapper::ContextMapper;
use std::collections::HashMap;
// use std::f64::consts::PI;
use super::utils;
// use super::super::context_mapper::Coord2D;
// use cairo::ScaledFont;
// use super::super::text::{FontData, draw_label};
use super::*;

#[derive(Debug)]
pub struct LineMapping {
    color : RGBA,
    x : Vec<f64>,
    y : Vec<f64>,
    width : f64,
    dash_n : i32,
    col_names : [String; 2],
    source : String
}

impl LineMapping {
    pub fn new(node : &Node) -> Self {
        let color = gdk::RGBA{
            red:0.0,
            green:0.0,
            blue:0.0,
            alpha : 0.0
        };
        let width = 1.0;
        let dash_n = 1;
        let x = Vec::<f64>::new();
        let y = Vec::<f64>::new();
        let col_names = [
            String::from("None"),
            String::from("None")
        ];
        let source = String::new();
        let mut mapping = LineMapping{color, x, y, width, dash_n, col_names, source};
        mapping.update_layout(node);
        mapping
    }

    fn build_dash(n : i32) -> Vec<f64> {
        let dash_sz = 10.0 / (n as f64);
        let mut dashes = Vec::<f64>::new();
        for _i in 1..n {
            dashes.push(dash_sz);
        }
    dashes
}

}

impl Mapping for LineMapping {

    fn draw(&self, mapper : &ContextMapper, ctx : &Context) {
        //println!("{:?}", self);
        ctx.save();
        ctx.set_source_rgb(
            self.color.red,
            self.color.green,
            self.color.blue
        );
        ctx.set_line_width(self.width);
        let dashes = LineMapping::build_dash(self.dash_n);
        ctx.set_dash(&dashes[..], 0.0);
        //println!("Received for drawing {:?} {:?}", self.x, self.y);
        let mut zip_xy = self.x.iter().zip(self.y.iter());
        let (mut prev_x, mut prev_y) = match zip_xy.next() {
            Some((prev_x, prev_y)) => (prev_x, prev_y),
            None => {
                ctx.restore();
                return;
            }
        };
        for (curr_x, curr_y) in zip_xy {
            if mapper.check_bounds(*curr_x, *curr_y) {
                let from = mapper.map(*prev_x, *prev_y);
                let to   = mapper.map(*curr_x, *curr_y);
                ctx.move_to(from.x, from.y);
                ctx.line_to(to.x, to.y);
                ctx.stroke();
            } else {
                //println!("Out of bounds mapping");
            }
            //println!("Now drawing to {:?} {:?}", to.x, to.y);
            prev_x = curr_x;
            prev_y = curr_y;
        }
        ctx.restore();
    }

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
        self.width = props["width"].parse().unwrap();
        self.dash_n = props["dash"].parse().unwrap();
        self.col_names[0] = props["x"].clone();
        self.col_names[1] = props["y"].clone();
    }

    fn properties(&self) -> HashMap<String, String> {
        let mut properties = MappingType::Line.default_hash();
        if let Some(e) = properties.get_mut("color") {
            *e = self.color.to_string();
        }
        if let Some(e) = properties.get_mut("width") {
            *e = self.width.to_string();
        }
        if let Some(e) = properties.get_mut("dash"){
            *e = self.dash_n.to_string();
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
        "line".into()
    }

    fn get_col_name(&self, col : &str) -> String {
        match col {
            "x" => self.col_names[0].clone(),
            "y" => self.col_names[1].clone(),
            _ => String::new()
        }
    }

    fn get_ordered_col_names(&self) -> Vec<(String, String)> {
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


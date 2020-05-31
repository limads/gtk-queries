use libxml::tree::node::Node;
use gdk::RGBA;
use cairo::Context;
use super::super::context_mapper::ContextMapper;
use std::collections::HashMap;
// use std::f64::consts::PI;
use super::utils;
//use super::context_mapper::Coord2D;
// use cairo::ScaledFont;
// use super::super::text::{FontData, draw_label};
use super::*;
use std::mem;

#[derive(Debug)]
pub struct BarMapping {
    color : RGBA,
    center_anchor : bool,
    x : Vec<f64>,
    y : Vec<f64>,
    h : Vec<f64>,
    w : Vec<f64>,
    col_names : [String; 4],
    bar_width : f64,
    origin : (f64, f64),
    bar_spacing : f64,
    horizontal : bool
}

impl BarMapping {

    pub fn new(node : &Node) -> Self {
        let color = gdk::RGBA{
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha : 0.0
        };
        let x = Vec::<f64>::new();
        let y = Vec::<f64>::new();
        let w = Vec::<f64>::new();
        let h = Vec::<f64>::new();
        let center_anchor = false;
        let col_names = [
            String::from("None"),
            String::from("None"),
            String::from("None"),
            String::from("None")
        ];
        let bar_width = 1.0;
        let origin = (0.0, 0.0);
        let bar_spacing = 1.0;
        let horizontal = false;
        let mut mapping = BarMapping{
            color, center_anchor, x, y, w, h,
            col_names, bar_width, origin, bar_spacing, horizontal
        };
        mapping.update_layout(node);
        mapping
    }

    fn adjust_bar(&mut self) {
        if self.horizontal {
            let n = self.w.len();
            self.y = (0..n).map(|i| self.origin.1 + self.bar_spacing * i as f64 ).collect();
            self.x = (0..n).map(|_| self.origin.0 ).collect();
            if self.center_anchor {
                let spacing = self.bar_spacing;
                self.y.iter_mut().for_each(|y| *y -= spacing / 2. );
            }
        } else {
            let n = self.h.len();
            self.y = (0..n).map(|_| self.origin.1 ).collect();
            self.x = (0..n).map(|i| self.origin.0  + self.bar_spacing * i as f64 ).collect();
            if self.center_anchor {
                let spacing = self.bar_spacing;
                self.x.iter_mut().for_each(|x| *x -= spacing / 2. );
            }
        }
        if self.horizontal {
            self.h = (0..self.w.len()).map(|_| self.bar_spacing * self.bar_width / 100. ).collect();
        } else {
            self.w = (0..self.h.len()).map(|_| self.bar_spacing * self.bar_width / 100. ).collect();
        }
    }

}


impl Mapping for BarMapping {

    fn draw(&self, mapper : &ContextMapper, ctx : &Context) {
        ctx.save();
        ctx.set_source_rgb(self.color.red, self.color.green, self.color.blue);
        //println!("Received for drawing {:?} {:?} {:?} {:?}", self.x, self.y, self.w, self.h);
        let r_iter = self.x.iter().zip(self.y.iter()
            .zip(self.w.iter()
            .zip(self.h.iter()))
        );
        for (x, (y, (w, h))) in r_iter {
            //let x_off = match self.center_anchor {
            //    false => *x,
            //    true => *x - *w / 2.0
            //};
            let tl_ok = mapper.check_bounds(*x, y + h);
            let tr_ok = mapper.check_bounds(x + w, y + h);
            let bl_ok = mapper.check_bounds(*x, *y);
            let br_ok = mapper.check_bounds(x + *w, *y);
            if  tl_ok && tr_ok && bl_ok && br_ok {
                let bottom_left = mapper.map(*x, *y);
                let bottom_right = mapper.map(x + *w, *y);
                let top_left = mapper.map(*x, *y + *h);
                //let top_right = mapper.map(x_off + *w, *y + *h);
                let coord_w = bottom_left.distance(bottom_right);
                let coord_h = bottom_left.distance(top_left);
                ctx.rectangle(top_left.x, top_left.y, coord_w, coord_h);
                ctx.fill();
                ctx.stroke();
            } else {
                println!("Out of bounds mapping");
            }
        }
        ctx.restore();
    }

    fn update_data(&mut self, mut values : Vec<Vec<f64>>) {
        //println!("Received for updating: {:?}", values);
        /*self.x = values[0].clone();
        self.y = values[1].clone();
        self.w = values[2].clone();
        self.h = values[3].clone();*/
        if self.horizontal {
            self.w = values.remove(0);
        } else {
            self.h = values.remove(0);
        }
        self.adjust_bar();
    }

    fn update_extra_data(&mut self, _values : Vec<Vec<String>>) {
        println!("Mapping has no extra data");
    }

    fn update_layout(&mut self, node : &Node) {
        let props = utils::children_as_hash(node, "property");
        self.color = props["color"].parse().unwrap();
        self.center_anchor = props["center_anchor"].parse().unwrap();
        self.col_names[0] = props["x"].clone();
        self.col_names[1] = props["y"].clone();
        self.col_names[2] = props["width"].clone();
        self.col_names[3] = props["height"].clone();
        self.origin.0 = props["origin_x"].parse().unwrap();
        self.origin.1 = props["origin_y"].parse().unwrap();
        self.bar_width = props["bar_width"].parse().unwrap();
        self.bar_spacing = props["bar_spacing"].parse().unwrap();
        let new_horiz = props["horizontal"].parse().unwrap();
        if self.horizontal != new_horiz {
            mem::swap(&mut self.w, &mut self.h);
            self.horizontal = new_horiz;
        }
        self.adjust_bar();
        println!("x: {:?}", self.x);
        println!("y: {:?}", self.y);
        println!("w: {:?}", self.w);
        println!("h: {:?}", self.h);
    }

    fn properties(&self) -> HashMap<String, String> {
        let mut properties = MappingType::Bar.default_hash();
        if let Some(e) = properties.get_mut("color") {
            *e = self.color.to_string();
        }
        if let Some(e) = properties.get_mut("center_anchor") {
            *e = self.center_anchor.to_string(); // verify if returns "true" "false" here
        }
        if let Some(e) = properties.get_mut("x") {
            *e = self.col_names[0].clone();
        }
        if let Some(e) = properties.get_mut("y") {
            *e = self.col_names[1].clone();
        }
        if let Some(e) = properties.get_mut("width") {
            *e = self.col_names[2].clone();
        }
        if let Some(e) = properties.get_mut("height") {
            *e = self.col_names[3].clone();
        }
        if let Some(e) = properties.get_mut("origin_x") {
            *e = self.origin.0.to_string();
        }
        if let Some(e) = properties.get_mut("origin_y") {
            *e = self.origin.1.to_string();
        }
        if let Some(e) = properties.get_mut("bar_width") {
            *e = self.bar_width.to_string();
        }
        if let Some(e) = properties.get_mut("bar_spacing") {
            *e = self.bar_spacing.to_string();
        }
        if let Some(e) = properties.get_mut("horizontal") {
            *e = self.horizontal.to_string();
        }
        properties
    }

    fn mapping_type(&self) -> String {
        "bar".into()
    }

    fn get_col_name(&self, col : &str) -> String {
        match col {
            "x" => self.col_names[0].clone(),
            "y" => self.col_names[1].clone(),
            "width" => self.col_names[2].clone(),
            "height" => self.col_names[3].clone(),
            _ => String::new()
        }
    }

    fn get_ordered_col_names(&self) -> Vec<(String, String)> {
        vec![
            (String::from("x"), self.get_col_name("x")),
            (String::from("y"), self.get_col_name("y")),
            (String::from("width"), self.get_col_name("width")),
            (String::from("height"), self.get_col_name("height"))
        ]
    }

    fn get_hash_col_names(&self) -> HashMap<String, String> {
        let mut cols = HashMap::new();
        cols.insert("x".into(), self.col_names[0].clone());
        cols.insert("y".into(), self.col_names[1].clone());
        cols.insert("width".into(), self.col_names[2].clone());
        cols.insert("height".into(), self.col_names[3].clone());
        cols
    }

    fn set_col_name(&mut self, col : &str, name : &str) {
        match col {
            "x" => { self.col_names[0] = name.into(); },
            "y" => { self.col_names[1] = name.into(); },
            "width" => { self.col_names[2] = name.into(); },
            "height" => { self.col_names[3] = name.into(); },
            _ => { }
        }
    }

    fn set_col_names(&mut self, cols : Vec<String>) -> Result<(), &'static str> {
        if cols.len() != 4 {
            Err("Wrong number of columns.")
        } else {
            self.set_col_name("x", &cols[0]);
            self.set_col_name("y", &cols[1]);
            self.set_col_name("width", &cols[2]);
            self.set_col_name("height", &cols[3]);
            Ok(())
        }
    }

}


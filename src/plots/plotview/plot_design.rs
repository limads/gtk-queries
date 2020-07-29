use gdk::RGBA;
use libxml::tree::node::Node;
use super::utils;
use std::error::Error;
use super::text;
use std::collections::HashMap;

pub struct PlotDesign {
    pub bg_color : RGBA,
    pub grid_color : RGBA,
    pub grid_width : i32,
    pub font : text::FontData
}

impl PlotDesign {

    pub fn new(node : &Node) -> Result<PlotDesign, Box<dyn Error>> {
        let design_props = utils::children_as_hash(node,"property");
        //println!("Design = {:?}", design_props);
        let standard_color = RGBA{red:0.0,green:0.0,blue:0.0,alpha:0.0};
        let bg_color = match design_props["bg_color"].parse() {
            Ok(c) => c,
            Err(_) => standard_color
         };
        let grid_color = match design_props["grid_color"].parse() {
            Ok(c) => c,
            Err(_) => standard_color
        };
        let grid_width : i32 = design_props["grid_width"].parse()?;
        let font = text::FontData::new_from_string(&design_props["font"]);
        let design = PlotDesign{
            bg_color,
            grid_color,
            grid_width,
            font
        };
        Ok(design)
    }

    pub fn description(&self) -> HashMap<String, String> {
        let mut desc = HashMap::new();
        desc.insert("bg_color".into(), self.bg_color.to_string());
        desc.insert("grid_color".into(), self.grid_color.to_string());
        desc.insert("grid_width".into(), self.grid_width.to_string());
        desc.insert("font".into(), self.font.description());
        desc
    }

    // Font pattern is assumed to be like
    // Monospace Regular 12
    /*fn update_colors(&self, bg : String, grid : String) {

    }*/
}


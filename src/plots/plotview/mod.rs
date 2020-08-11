use cairo::Context;
use libxml::tree::document::{Document, SaveOptions};
use libxml::parser::Parser;
use libxml::tree::node::Node;
use std::default::Default;
use std::collections::HashMap;
use std::error::Error;
use gtk::WidgetExt;
use std::rc::*;
use std::cell::*;
use std::result::Result;
use std::io::{ErrorKind, Write};
use mappings::area::*;
use mappings::*;
use context_mapper::*;
use grid_segment::*;
use plot_design::*;
use std::fmt::Display;
mod text;
use std::any::Any;
use std::error;
use std::{fmt, fs::File};
use cairo::SvgSurface;

pub mod mappings;

pub mod plot_view;

pub mod context_mapper;

pub mod plot_design;

pub mod grid_segment;

use mappings::bar::*;

use mappings::scatter::*;

use mappings::line::*;

use mappings::surface::*;

use mappings::text::*;

//use sync::*;
/*impl Mapping for BarMapping {
}*/

/*#[derive(Debug)]
pub enum PlotError {
    PropertyNotFound,
    ViolateBounds
}

impl Display for PlotError {

}

impl Error for PlotError {

}*/

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupSplit {
    Unique,
    Vertical,
    Horizontal,
    Four,
    ThreeLeft,
    ThreeTop,
    ThreeRight,
    ThreeBottom
}

/// A plotgroup has at least one first plot.
pub struct PlotGroup {

    design : PlotDesign,

    plots : Vec<PlotArea>,

    split : GroupSplit,

    h_ratio : f64,

    v_ratio : f64,

    parser : Parser,

    doc : Document
}

impl PlotGroup {

    pub fn new(layout_path : String) -> Result<Self, String> {
        let plots = Vec::new();
        let parser : Parser = Default::default();
        let doc = parser.parse_file(&layout_path)
            .map_err(|e| format!("Failed parsing XML file: {}", e) )?;
        let root = doc.get_root_element().unwrap();
        let design_node = root
            .findnodes("object[@class='design']")
            .expect("No design node")
            .first().cloned().expect("No design node");
        let design = PlotDesign::new(&design_node)
            .expect("Failed instantiating design");
        let mut plot_group = Self{ parser, doc, plots, split : GroupSplit::Unique, v_ratio : 1.0, h_ratio : 1.0, design };
        plot_group.load_layout(layout_path)?;
        Ok(plot_group)
    }

    pub fn draw_to_file(&mut self, path : &str, w : usize, h : usize) {
        // TODO Error creating SVG surface: "error while writing to output stream
        let surf = SvgSurface::new(w as f64, h as f64, Some(path))
            .expect("Error creating SVG surface");
        let ctx = Context::new(&surf);
        self.draw_to_context(&ctx, 0, 0, w as i32, h as i32);
    }

    pub fn size(&self) -> usize {
        self.plots.len()
    }

    /// Draws the current Plot definition to a Cairo context.
    /// Used internally by PlotView to draw to the context
    /// of a gtk::DrawingArea. Users can also retrive the context
    /// from cairo::ImageSurface::create() to plot directly to
    /// SVG/PNG/PDF files.
    pub fn draw_to_context(
        &mut self,
        ctx : &Context,
        x : i32,
        y : i32,
        w : i32,
        h : i32
    ) {
        let top_left = (0.05, 0.05);
        let top_right = (w as f64 * self.v_ratio, 0.05);
        let bottom_left = (0.05, h as f64 * self.h_ratio);
        let bottom_right = (w as f64 * self.v_ratio, h as f64 * self.h_ratio);
        for (i, plot) in self.plots.iter_mut().enumerate() {
            let origin_offset = match (&self.split, i) {
                (GroupSplit::Horizontal, 1) => (0.05, h as f64 * self.v_ratio),
                (GroupSplit::Vertical, 1) => (w as f64 * self.h_ratio, 0.05),
                (GroupSplit::Four, 1) => top_right,
                (GroupSplit::Four, 2) => bottom_left,
                (GroupSplit::Four, 3) => bottom_right,
                (GroupSplit::ThreeLeft, 1) => top_right,
                (GroupSplit::ThreeLeft, 2) => bottom_right,
                (GroupSplit::ThreeTop, 1) => bottom_left,
                (GroupSplit::ThreeTop, 2) => bottom_right,
                (GroupSplit::ThreeRight, 0) => top_left,
                (GroupSplit::ThreeRight, 1) => top_left,
                (GroupSplit::ThreeRight, 2) => bottom_left,
                (GroupSplit::ThreeBottom, 0) => top_left,
                (GroupSplit::ThreeBottom, 1) => bottom_left,
                (GroupSplit::ThreeBottom, 2) => bottom_right,
                _ => top_left
            };

            let h_full_v = (1., self.v_ratio);
            let h_full_v_compl = (1., 1. - self.v_ratio);
            let h_v_full = (self.h_ratio, 1.);
            let h_compl_v_full = (1. - self.h_ratio, 1.);
            let h_compl_v = (1. - self.h_ratio, self.v_ratio);
            let h_v_compl = (self.h_ratio, 1. - self.v_ratio);
            let diag = (self.h_ratio, self.v_ratio);
            let diag_compl = (1. - self.h_ratio, 1. - self.v_ratio);
            let scale_factor = match (&self.split, i) {
                (GroupSplit::Horizontal, 0) => h_full_v,
                (GroupSplit::Horizontal, 1) => h_full_v_compl,
                (GroupSplit::Vertical, 0) => h_full_v,
                (GroupSplit::Vertical, 1) => h_compl_v_full,
                (GroupSplit::Four, 0) => diag,
                (GroupSplit::Four, 1) => h_compl_v,
                (GroupSplit::Four, 2) => h_v_compl,
                (GroupSplit::Four, 3) => diag_compl,
                (GroupSplit::ThreeLeft, 0) => h_full_v,
                (GroupSplit::ThreeLeft, 1) => h_compl_v,
                (GroupSplit::ThreeLeft, 2) => diag_compl,
                (GroupSplit::ThreeTop, 0) => h_compl_v_full,
                (GroupSplit::ThreeTop, 1) => h_v_compl,
                (GroupSplit::ThreeTop, 2) => diag_compl,
                (GroupSplit::ThreeRight, 0) => diag,
                (GroupSplit::ThreeRight, 1) => h_full_v_compl,
                (GroupSplit::ThreeRight, 2) => h_v_compl,
                (GroupSplit::ThreeBottom, 0) => diag,
                (GroupSplit::ThreeBottom, 1) => h_compl_v,
                (GroupSplit::ThreeBottom, 2) => h_compl_v_full,
                _ => (1., 1.)
            };
            let origin = (x as f64 + origin_offset.0, y as f64 + origin_offset.1);
            let size = ((w as f64 * scale_factor.0) as i32, (h as f64 * scale_factor.1) as i32);
            ctx.save();
            // ctx.move_to(0.0, 0.0);
            ctx.translate(origin.0, origin.1);
            // println!("i: {}; origin: {:?}, size: {:?}", i, origin, size);
            plot.draw_plot(&ctx, &self.design, size.0, size.1);
            ctx.restore();
        }
        //println!("--");
    }

    pub fn reload_layout_data(&mut self) -> Result<(), Box<dyn Error>> {
        let _root_el = self.doc.get_root_element()
            .expect("Root node not found");
        for plot in /*root_el.get_child_nodes().iter().zip(*/ self.plots.iter_mut() {
            plot.reload_layout_node( /*node.clone()*/ )?;
        }
        Ok(())
    }

    pub fn load_layout(&mut self, path : String) -> Result<(), String> {

        use GroupSplit::*;

        self.doc = self.parser.parse_file(&path)
            .map_err(|e| format!("Failed parsing XML: {}", e) )?;
        let root_el = self.doc.get_root_element()
            .ok_or(format!("Root node not found"))?;
        if &root_el.get_name()[..] != "plotgroup" {
            return Err(format!("Root node should be called plotgroup"));
        }
        self.plots.clear();
        let mut found_split = false;
        for node in root_el.get_child_nodes() {
            //println!("Node name: {}", node.get_name());
            if &node.get_name()[..] == "property" {
                //println!("Property: {:?}", node.get_attribute("name"));
                match node.get_attribute("name").as_ref().and_then(|s| Some(&s[..]) ) {
                    Some("vertical_ratio") => {
                        self.v_ratio = node.get_content().parse()
                            .map_err(|_| format!("Unabe to parse vertical ratio"))?;
                    },
                    Some("horizontal_ratio") => {
                        self.h_ratio = node.get_content().parse()
                            .map_err(|_| format!("Unabe to parse horizontal ratio"))?;
                    },
                    Some("split") => {
                        found_split = true;
                        match &node.get_content()[..] {
                            "Unique" => self.split = Unique,
                            "Four" => self.split = Four,
                            "Horizontal" => self.split = Horizontal,
                            "Vertical" => self.split = Vertical,
                            "ThreeLeft" => self.split = ThreeLeft,
                            "ThreeTop" => self.split = ThreeTop,
                            "ThreeRight" => self.split = ThreeRight,
                            "ThreeBottom" => self.split = ThreeBottom,
                            _ => return Err(String::from("Unrecognized split value"))
                        }
                    },
                    _ => return Err(String::from("Unknown property"))
                }
            }
            if &node.get_name()[..] == "plotarea" {
                self.plots.push(PlotArea::new(node.clone()));
            }
        }
        if self.plots.len() == 0 {
            return Err("Root node plotgroup does not contain any plotarea children.".into());
        }
        if !found_split {
            self.split = Unique;
        }
        match self.split {
            Unique => if self.plots.len() != 1 {
                return Err("'None' split require 1 plot".into());
            },
            Vertical => if self.plots.len() != 2 {
                return Err("Vertical split require 2 plots".into());
            },
            Horizontal => if self.plots.len() != 2 {
                return Err("Horizontal split require 2 plots".into());
            },
            Four => if self.plots.len() != 4 {
                return Err("'Both' split require 4 plots".into());
            },
            ThreeLeft | ThreeTop | ThreeRight | ThreeBottom => if self.plots.len() != 3 {
                return Err("'Three' split require 3 plots".into());
            },
            _ => unimplemented!()
        }
        self.reload_layout_data()
            .map_err(|_| "Could not reload layout data")?;
        for plot in self.plots.iter_mut() {
            plot.reload_mappings()
                .map_err(|()| "Could not reload mappings from informed layout")?;
        }
        // println!("h: {}; v : {}; split: {:?}", self.h_ratio, self.v_ratio, self.split);
        Ok(())
        // Document.get_root_element(&self) -> Option<Node>
    }

    pub fn save_layout(&self, path : String) {
        let content = self.get_layout_as_text();
        match File::create(path) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(content.as_bytes()) {
                    println!("Error writing to file: {}", e);
                }
            },
            Err(e) => println!("Error creating file: {}", e)
        }
        //self.doc.save_file(&path)
        //    .expect("Could not save file");
    }

    pub fn get_layout_as_text(&self) -> String {
        let mut opts : SaveOptions = Default::default();
        opts.format = true;
        //self.doc.to_string(opts)
        self.doc.to_string_with_options(opts)
    }

    pub fn update_design(&mut self, property : &str, value : &str) {
        println!("Updating design at {} to {}", property, value);
        if property.is_empty() || value.is_empty() {
            println!("Informed empty property!");
            return;
        }
        let root = self.doc.get_root_element().unwrap();
        let design_node = root
            .findnodes("object[@class='design']")
            .expect("No design node")
            .first().cloned().expect("No design node");
        match design_node.findnodes(&property) {
            Ok(mut props) => {
                if let Some(p) = props.iter_mut().next() {
                    if let Err(e) = p.set_content(&value) {
                        println!("Error setting node content: {}", e);
                        return;
                    }
                    self.design = PlotDesign::new(&design_node)
                        .expect("Failed loading plot design");
                } else {
                    println!("No property named {} found", property);
                }
            },
            _ => { println!("Failed at finding property {}", property); }
        }
    }

    pub fn update_plot_property(&mut self, ix: usize, property : &str, value : &str) {
        println!("Updating {} at {} to {}", ix, property, value);
        self.plots[ix].update_layout(property, value);
    }

    pub fn update_mapping(&mut self, ix : usize, id : &str, data : &Vec<Vec<f64>>) -> Result<(), Box<dyn Error>> {
        println!("Updating {} at {} to {:?}", ix, id, data);
        self.plots[ix].update_mapping(id, data)
    }

    pub fn update_mapping_text(&mut self, ix : usize, id : &str, text : &Vec<String>) -> Result<(), Box<dyn Error>> {
        self.plots[ix].update_mapping_text(id, text)
    }

    pub fn update_mapping_columns(&mut self, ix : usize, id : &str, cols : Vec<String>) -> Result<(), Box<dyn Error>> {
        self.plots[ix].update_mapping_columns(id, cols)
    }

    pub fn update_source(&mut self, ix : usize, id : &str, source : String) -> Result<(), Box<dyn Error>> {
        self.plots[ix].update_source(id, source)
    }

    pub fn add_mapping(&mut self, ix : usize, mapping_index : String, mapping_type : String) -> Result<(), String> {
        self.plots[ix].add_mapping(mapping_index, mapping_type, &self.doc)
    }

    pub fn remove_mapping(&mut self, ix : usize, id : &str) {
        self.plots[ix].remove_mapping(id);
    }

    pub fn scale_info(&self, ix : usize, scale : &str) -> HashMap<String, String> {
        self.plots[ix].scale_info(scale)
    }

    pub fn design_info(&self) -> HashMap<String, String> {
        self.design.description()
    }

    pub fn mapping_info(&self, ix : usize) -> Vec<(String, String, HashMap<String,String>)> {
        self.plots[ix].mapping_info()
    }

    pub fn group_split(&self) -> GroupSplit {
        self.split.clone()
    }

}

pub struct PlotArea {
    mappings : Vec<Box<dyn Mapping>>,
    mapper : ContextMapper,
    x : GridSegment,
    y : GridSegment,
    frozen : bool,
    node : Node
}

#[derive(Debug)]
pub enum PlotError {
    InvalidData(&'static str),
    OutOfBounds(&'static str),
    Other(&'static str)
}

impl PlotError {
    pub fn new() -> Self {
        Self::Other("Unknown error")
    }
}

impl Display for PlotError {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidData(msg) => { write!(f, "{}", msg) },
            Self::OutOfBounds(msg) => { write!(f, "{}", msg) },
            Self::Other(msg) => { write!(f, "{}", msg) }
        }
    }

}

impl error::Error for PlotError {

}

impl PlotArea {

    pub fn new(node : Node) -> PlotArea {
        let mappings = Vec::new();
        let mapper : ContextMapper = Default::default();
        let x : GridSegment = Default::default();
        let y : GridSegment = Default::default();
        let frozen = false;
        let pl_area =
            PlotArea{ mappings, mapper, x, y, frozen, node };
        // if let Err(e) = pl_area.reload_layout_data() {
        //   println!("Error when reloading layout data: {}", e.description());
        // }
        pl_area
    }

    /*fn save(&self, path : String, w : i32, h : i32) {

    }*/

    fn draw_plot(&mut self, ctx: &Context, design : &PlotDesign, w : i32, h : i32) {
        self.mapper.update_dimensions(w, h);
        // If frozen, do not redraw background/grid.
        // Draw only frozen mapping increment.
        self.draw_background(ctx, design);
        self.draw_grid(ctx, design);
        for mapping in self.mappings.iter() {
            // println!("Mapping drawn");
            mapping.draw(&self.mapper, &ctx);
        }
    }

    //fn on_draw(&self, ctx : &Context) {
    //}

    pub fn freeze_at_mapping(&mut self, _mapping : &str) -> Result<(),()> {
        // Call mapping.setup
        // set frozen to true
        Err(())
    }

    pub fn unfreeze(&mut self) {
        self.frozen = false;
    }

    // let props = ["from", "to", "n_intervals", "invert", "log_scaling"];

    // TODO improve error handling here.
    fn read_grid_segment(
        &self,
        props : HashMap<String, String>
    ) -> Result<GridSegment, Box<dyn Error>> {
        let from : f64 = props.get("from").unwrap().parse()?;
        let to : f64 = props["to"].parse()?;
        let nint : i32 = props["n_intervals"].parse()?;
        let offset : i32 = props["grid_offset"].parse()?;
        let invert : bool = props["invert"].parse()?;
        let log : bool = props["log_scaling"].parse()?;
        let precision : i32 = props["precision"].parse()?;
        let label = props["label"].clone();
        Ok( GridSegment::new(
            label, precision, from, to, nint, log, invert, offset) )
    }

    /// Reloads all mappings from XML definition,
    /// clearing any existent data.
    pub fn reload_mappings(&mut self) -> Result<(),()> {
        // let root = self.doc.get_root_element()
        //    .expect("Root node not found");
        self.mappings.clear();
        if let Ok(mappings) = self.node.findnodes("object[@class='mapping']") {
            //println!("mappings to add -> {:?}", mappings);
            for mapping_node in mappings {
                let mapping_ix = mapping_node
                    .get_attribute("index").expect("No attr");
                let mapping_type = mapping_node
                    .get_attribute("type").expect("No attr");
                let mapping : Option<Box<dyn Mapping>> = match &mapping_type[..] {
                    "line" => Some( Box::new(LineMapping::new(&mapping_node)) ),
                    "scatter" => Some( Box::new(ScatterMapping::new(&mapping_node)) ),
                    "bar" => Some( Box::new(BarMapping::new(&mapping_node)) ),
                    "text" => Some( Box::new(TextMapping::new(&mapping_node)) ),
                    "area" => Some( Box::new(AreaMapping::new(&mapping_node)) ),
                    "surface" => Some( Box::new(SurfaceMapping::new(&mapping_node)) ),
                    _ => { println!("Unrecognized mapping"); None }
                };
                if let Some(m) = mapping{
                    self.mappings.insert(mapping_ix.parse::<usize>().unwrap(), m);
                } else {
                    println!("Unknown mapping added");
                    return Err(());
                }
            }
        } else {
            println!("No mappings to load");
        }
        Ok(())
    }

    /// Parses the XML file definition at self.doc
    /// and updates all layout information used for plotting.
    /// Does not mess with mapping data.
    pub fn reload_layout_node(&mut self /*, node : Node*/ ) -> Result<(), Box<dyn Error>> {
        // TODO confirm this does not need to be reset here.
        // self.node = node;
        // let root = self.doc.get_root_element()
        //    .expect("Root node not found");
        println!("updating node: {:?} position: {:?}", self.node.get_name(), self.node.get_property("position"));

        let xprops = utils::children_as_hash(
            &self.node, "object[@name='x']/property");
        println!("xprops: {:?}", xprops);
        let yprops = utils::children_as_hash(
            &self.node, "object[@name='y']/property");
        self.x = self.read_grid_segment(xprops)?;
        println!("x grid: {:?}", self.x);
        self.y = self.read_grid_segment(yprops)?;
        self.mapper = ContextMapper::new(self.x.from, self.x.to,
            self.y.from, self.y.to, self.x.log, self.y.log,
            self.x.invert, self.y.invert);
        Ok(())
    }

    fn new_base_mapping_node(
        &self,
        mapping_type : &str,
        mapping_index : &str,
        doc : &Document
    ) -> Result<Node,Box<dyn Error>> {
        let mut new_mapping = Node::new("object", Option::None, &doc)
            .unwrap();
        new_mapping.set_attribute("class", "mapping")?;
        new_mapping.set_attribute("type", &mapping_type)?;
        new_mapping.set_attribute("index", &mapping_index)?;

        /*let mut color_prop = Node::new(
            "property", Option::None, &self.doc)
            .unwrap();
        color_prop.set_attribute("name", "color")?;
        color_prop.set_content("#000000")?;
        new_mapping.add_child(&mut color_prop)?;*/
        Ok(new_mapping)
    }

    pub fn add_mapping(
        &mut self,
        mapping_index : String,
        mapping_type : String,
        doc : &Document
    ) -> Result<(), String> {
        //let mut root = self.doc.get_root_element().expect("No root");
        let mut new_mapping = self.new_base_mapping_node(
            &mapping_type[..],
            &mapping_index[..],
            &doc
        ).map_err(|e| format!("{}", e) )?;
        if let Some(mtype) = MappingType::from_str(&mapping_type[..]) {
            utils::populate_node_with_hash(
                &doc,
                &mut new_mapping,
                mtype.default_hash()
            ).map_err(|e| format!("{}", e) )?;
            let m_ix : usize = mapping_index.parse::<usize>().map_err(|e| format!("{}", e) )?;
            if m_ix > self.mappings.len() {
                return Err(format!(
                    "Tried to insert mapping at position {}, but plot has only {} elements",
                    m_ix, self.mappings.len()
                ));
            }
            match mtype {
                MappingType::Line => {
                    let line_mapping = LineMapping::new(&new_mapping);
                    self.mappings.insert(m_ix, Box::new(line_mapping));
                },
                MappingType::Scatter => {
                    let scatter_mapping = ScatterMapping::new(&new_mapping);
                    self.mappings.insert(m_ix, Box::new(scatter_mapping));
                },
                MappingType::Bar => {
                    let bar_mapping = BarMapping::new(&new_mapping);
                    self.mappings.insert(m_ix, Box::new(bar_mapping));
                },
                MappingType::Text => {
                    let text_mapping = TextMapping::new(&new_mapping);
                    self.mappings.insert(m_ix, Box::new(text_mapping));
                },
                MappingType::Area => {
                    let area_mapping = AreaMapping::new(&new_mapping);
                    self.mappings.insert(m_ix, Box::new(area_mapping));
                },
                MappingType::Surface => {
                    let surface_mapping = SurfaceMapping::new(&new_mapping);
                    self.mappings.insert(m_ix, Box::new(surface_mapping));
                }
            }
            self.node.add_child(&mut new_mapping)?;

            // TODO verify if this is necessary!!
            // self.doc.set_root_element(&root);
        } else {
            return Err(format!("Unrecognized mapping {}", mapping_type));
        }

        /*if mapping_type == "line" {
            let mtype = MappingType::Line;

            let mut width_property = Node::new(
                "property", Option::None, &self.doc).unwrap();
            let mut dash_property = Node::new(
                "property", Option::None, &self.doc).unwrap();
            width_property.set_attribute("name", "width")?;
            dash_property.set_attribute("name", "dash")?;
            width_property.set_content("1")?;
            dash_property.set_content("1")?;
            new_mapping.add_child(&mut width_property)?;
            new_mapping.add_child(&mut dash_property)?;
            let line_mapping = LineMapping::new(&new_mapping);
            println!("{:?}", mapping_name.clone());
            self.mappings.insert(mapping_name, Box::new(line_mapping));
            root.add_child(&mut new_mapping)?;
            self.doc.set_root_element(&root);
        }*/
        //if self.reload_layout_data().is_err() {
        //    println!("Problem reloading data after adding new mapping");
        //}

        Ok(())
    }

    fn accomodate_dimension(
        &mut self,
        data : &[f64],
        old_min : f64,
        old_max : f64,
        dim_name : &str
    ) {
        let new_min = data.iter().fold(old_min, |min, el| {
            if *el < min {
                *el
            } else {
                min
            }
        });
        let new_max = data.iter().fold(old_max, |max, el| {
            if *el > max {
                *el
            } else {
                max
            }
        });
        if new_min < old_min {
            self.update_layout(
                &format!("object[@name='{}']/property[@name='from']", dim_name)[..],
                &new_min.to_string()
            );
        }
        if new_max > old_max {
            self.update_layout(
                &format!("object[@name='{}']/property[@name='to']", dim_name)[..],
                &new_max.to_string()
            );
        }
    }

    pub fn update_mapping(
        &mut self,
        id : &str,
        data : &Vec<Vec<f64>>
    ) -> Result<(), Box<dyn Error>> {
        if data.len() < 1 {
            return Err(Box::new(PlotError::InvalidData("Invalid data")))
        }
        let (xmin, xmax, ymin, ymax) = self.mapper.data_extensions();
        if data.len() == 1 {
            self.accomodate_dimension(&data[0][..], ymin, ymax, "y");
        } else {
            self.accomodate_dimension(&data[0][..], xmin, xmax, "x");
            self.accomodate_dimension(&data[1][..], ymin, ymax, "y");
        }
        if let Some(mapping) = self.mappings.get_mut(id.parse::<usize>().unwrap()) {
            mapping.update_data(data.clone());
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                ErrorKind::Other,
                "Cannot recover mapping "
            )))
        }
    }

    pub fn remove_mapping(&mut self, id : &str) {
        let n = self.mappings.len();
        let pos = id.parse::<usize>().unwrap();
        //let mut root = self.doc.get_root_element().expect("No root at remove");
        let xpath = String::from("object[@index='") + id +  "']";
        println!("{}", xpath);
        let mut nodes = self.node.findnodes(&xpath[..]).expect("No node with informed id");
        let node = nodes.get_mut(0).expect("No first node with informed id");
        node.unlink_node();
        for i in (pos + 1)..n {
            let xpath = String::from("object[@index='") + &i.to_string()[..] +  "']";
            let mut nodes = self.node.findnodes(&xpath[..]).expect("No node with informed id");
            let node = nodes.get_mut(0).expect("No first node with informed id");
            node.set_attribute("index", &((i - 1).to_string())[..]).expect("No index property");
        }
        self.mappings.remove(pos);
        for m in self.mappings.iter() {
            println!("Current mappings: {:?}", m.mapping_type());
        }
    }

    pub fn update_mapping_text(
        &mut self,
        id : &str,
        text : &Vec<String>
    ) -> Result<(), Box<dyn Error>> {
        if let Some(mapping) = self.mappings.get_mut(id.parse::<usize>().unwrap()) {
            mapping.update_extra_data(vec![text.clone()]);
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                ErrorKind::Other,
                "Unable to update text mapping position"
            )))
        }

        /*
            // println!("{}, {:?}", mapping.mapping_type(), mapping.properties());
            {
            // let mapping = mapping as &mut dyn Any;
            // println!("{:?}", (mapping as &mut dyn Any).type_id());
            match (mapping as &mut dyn Any).downcast_mut::<TextMapping>() {
                Some(m) => {
                    m.set_text_data(&text);
                    Ok(())
                },
                None => {
                    Err(Box::new(std::io::Error::new(
                        ErrorKind::Other,
                        "Informed mapping does not support text update"
                    )))
                }
            }
            }
        } else {
            Err(Box::new(std::io::Error::new(
                ErrorKind::Other, "Cannot recover mapping")))
        }*/
    }

    /* Given a resolvable full path to a property, update it. */
    pub fn update_layout(&mut self, property : &str, value : &str) {
        // let root = self.doc.get_root_element().expect("No root");
        // println!("{} : {}", property, value);
        if property.is_empty() || value.is_empty() {
            println!("Informed empty property!");
            return;
        }
        match self.node.findnodes(&property) {
            Ok(mut props) => {
                if let Some(p) = props.iter_mut().next() {
                    if let Err(e) = p.set_content(&value) {
                        println!("Error setting node content: {}", e);
                    }
                    println!("new node content: {:?}, {:?}", p.get_property("name"), p.get_content());
                    //println!("new node at root: {:?}", self.node.get_content());
                    let parent = p.get_parent().unwrap();
                    match parent.get_attribute("class") {
                        Some(ref class) if class == "mapping" => {
                            if let Some(index) = parent.get_attribute("index") {
                                if let Some(m) = self.mappings.get_mut(index.parse::<usize>().unwrap()) {
                                    m.update_layout( &parent );
                                } else {
                                    println!("No mapping at {} available", index);
                                }
                            } else {
                                println!("Invalid mapping index");
                            }
                        },
                        Some(ref class) if class != "mapping" => {
                            println!("Updated property: {:?}", self.node.findnodes(property).unwrap().iter().next().unwrap().get_content());
                            if let Err(e) = self.reload_layout_node() {
                                println!("Could not apply property {} ({})", property, e);
                            }
                            println!("Updated property after reload: {:?}", self.node.findnodes(property).unwrap().iter().next().unwrap().get_content());
                        },
                        _ => {
                            println!("Layout item missing class attribute.");
                        }
                    }
                } else {
                    println!("{}", "Property ".to_owned() + property + " not found!");
                }
            },
            Err(e) => {
                println!("No property {} found at node {:?} ({:?})", property, self.node, e);
            }
        }
    }

    pub fn clear_all_data(&mut self) {
        for m in self.mappings.iter_mut() {
            let mut empty_data : Vec<Vec<f64>> = Vec::new();
            empty_data.push(Vec::new());
            empty_data.push(Vec::new());
            match &m.mapping_type()[..] {
                "line" => {

                },
                "scatter" => {

                },
                "bar" => {
                    empty_data.push(Vec::new());
                    empty_data.push(Vec::new());
                },
                "area" => {
                    empty_data.push(Vec::new());
                },
                "text" => {
                    //TODO clear text
                    match (m as &mut dyn Any).downcast_mut::<TextMapping>() {
                        Some(m) => {
                            m.set_text_data(&Vec::new());
                        },
                        _ => { println!("Could not downcast to text when clearing its data"); }
                    }
                },
                "surface" => {
                    empty_data.push(Vec::new());
                }
                _ => {
                    println!("Invalid mapping type");
                    return;
                }
            }
            m.update_data(empty_data);
        }
    }

    fn draw_background(&self, ctx : &Context, design : &PlotDesign) {
        ctx.save();
        ctx.set_line_width(0.0);
        ctx.set_source_rgb(
            design.bg_color.red,
            design.bg_color.green,
            design.bg_color.blue);
        ctx.rectangle(
            0.1*(self.mapper.w as f64), 0.1*(self.mapper.h as f64),
            0.8*(self.mapper.w as f64), 0.8*(self.mapper.h as f64));
        ctx.fill();
        ctx.restore();
    }

    fn draw_grid_line(
        &self,
        ctx : &Context,
        design : &PlotDesign,
        from : Coord2D,
        to : Coord2D
    ) {
        ctx.save();
        ctx.set_source_rgb(
            design.grid_color.red,
            design.grid_color.green,
            design.grid_color.blue);
        ctx.move_to(from.x, from.y);
        ctx.line_to(to.x, to.y);
        ctx.stroke();

        //ctx.set_source_rgb(0.2666, 0.2666, 0.2666);
        //ctx.move_to(from.x + label_off_x, from.y + label_off_y);
        //ctx.show_text(&label);
        //self.draw_centered_label(ctx, &label, Coord2D::new(from.x + label_off_x, from.y + label_off_y), false);
        //self.draw_grid_value(ctx, &label)
        ctx.restore();
    }

    /// Since the y value is always centered, this function accepts the option
    /// to center the x value (true for the x labels; false for the y labels).
    fn draw_grid_value(
        &self,
        ctx : &Context,
        design : &PlotDesign,
        value : &str,
        pos : Coord2D,
        center_x : bool,
        ext_off_x : f64,
        ext_off_y : f64
    ) {
        ctx.set_source_rgb(0.2666, 0.2666, 0.2666);
        text::draw_label(
            &design.font.sf,
            ctx,
            &value[..],
            pos,
            false,
            (center_x, true),
            Some(ext_off_x),
            Some(ext_off_y)
        );
    }

    pub fn steps_to_labels(
        steps : &[f64],
        precision : usize
    ) -> Vec<String> {
        steps.iter()
            .map(|s| format!("{:.*}", precision, s))
            .collect()
    }

    fn get_max_extent(
        &self,
        design : &PlotDesign,
        labels : &Vec<String>
    ) -> f64 {
        labels.iter()
            .map(|l| design.font.sf.text_extents(&l[..]).x_advance)
            .fold(0.0, |m, f| f64::max(m,f))
    }

    /*fn shift_coord_by_max_extent(
        base_coord : Coord2D,
        max_extent : f64
    ) -> Coord2D {

            .collect()
    }*/

    fn draw_grid(&self, ctx : &Context, design : &PlotDesign) {
        ctx.save();
        ctx.set_line_width(design.grid_width as f64);
        design.font.set_font_into_context(&ctx);
        let mut x_labels = PlotArea::steps_to_labels(
            &self.x.steps[..],
            self.x.precision as usize
        );
        if self.mapper.xinv {
            x_labels.reverse();
        }
        for (x, x_label) in self.x.steps.iter().zip(x_labels.iter()) {
            let from = match (self.mapper.xinv, self.mapper.yinv) {
                (false, false) => self.mapper.map(*x, self.mapper.ymin),
                (false, true) => self.mapper.map(*x, self.mapper.ymax),
                (true, false) => self.mapper.map(self.mapper.xmin + self.mapper.xmax - *x, self.mapper.ymin),
                (true, true) => self.mapper.map(self.mapper.xmin + self.mapper.xmax - *x, self.mapper.ymax)
            };
            let to = match (self.mapper.xinv, self.mapper.yinv) {
                (false, false) => self.mapper.map(*x, self.mapper.ymax),
                (false, true) => self.mapper.map(*x, self.mapper.ymin),
                (true, false) =>  self.mapper.map(self.mapper.xmin + self.mapper.xmax - *x, self.mapper.ymax),
                (true, true) => self.mapper.map(self.mapper.xmin + self.mapper.xmax - *x, self.mapper.ymin)
            };
            // let from = self.mapper.map(*x, self.mapper.ymin);
            // let to = match self.mapper.self.mapper.map(*x, self.mapper.ymax);
            // println!("{:?}, {:?}, {:?}", x, from, to);
            self.draw_grid_line(ctx, design, from, to);
            self.draw_grid_value(ctx, design, x_label, from, true, 0.0, 1.5);
        }

        let mut y_labels = PlotArea::steps_to_labels(
            &self.y.steps[..],
            self.y.precision as usize
        );
        if self.mapper.yinv {
            y_labels.reverse();
        }
        let max_extent = self.get_max_extent(design, &y_labels);
        for (y, y_label) in self.y.steps.iter().zip(y_labels.iter()) {
            let mut from = match (self.mapper.xinv, self.mapper.yinv) {
                (false, false) => self.mapper.map(self.mapper.xmin, *y),
                (false, true) => self.mapper.map(self.mapper.xmin, self.mapper.ymin + self.mapper.ymax - *y),
                (true, false) => self.mapper.map(self.mapper.xmax, *y),
                (true, true) => self.mapper.map(self.mapper.xmax, self.mapper.ymin + self.mapper.ymax - *y)
            };
            let to = match (self.mapper.xinv, self.mapper.yinv) {
                (false, false) => self.mapper.map(self.mapper.xmax, *y),
                (false, true) => self.mapper.map(self.mapper.xmax, self.mapper.ymin + self.mapper.ymax - *y),
                (true, false) =>  self.mapper.map(self.mapper.xmin, *y),
                (true, true) => self.mapper.map(self.mapper.xmin, self.mapper.ymin + self.mapper.ymax - *y)
            };
            self.draw_grid_line(ctx, design, from, to);
            //let mut y_label_coord = match self.mapper.yinv {
            //    true => to,
            //    false => from
            //};
            from.x -= 1.1*max_extent;
            self.draw_grid_value(ctx, design, y_label, from, false, 0.0, 0.0);
        }
        self.draw_scale_names(ctx, design);
        ctx.restore();
    }

    fn draw_scale_names(&self, ctx : &Context, design : &PlotDesign) {
        let pos_x = Coord2D::new(
            self.mapper.w as f64 * 0.5,
            self.mapper.h as f64 * 0.975
        );
        let pos_y = Coord2D::new(
            self.mapper.w as f64 * 0.025,
            self.mapper.h as f64 * 0.5
        );
        text::draw_label(
            &design.font.sf,
            ctx,
            &self.x.label[..],
            pos_x,
            false,
            (true, true),
            None,
            None
        );
        text::draw_label(
            &design.font.sf,
            ctx,
            &self.y.label[..],
            pos_y,
            true,
            (true, true),
            None,
            None
        );
    }

    /*fn update_mapping_name(name : &str) {
        // Verify if mapping name is not x|y|design|
    }*/

    /// For each mapping, return a tuple with (name, type, properties).
    pub fn mapping_info(&self) -> Vec<(String, String, HashMap<String,String>)> {
        let mut info = Vec::new();
        for (i, m) in self.mappings.iter().enumerate() {
            info.push((i.to_string(), m.mapping_type(), m.properties()))
        }
        //println!("{:?}", info);
        info
    }

    pub fn mapping_column_names(&self, id : &str) -> Vec<(String, String)> {
        let mut names = Vec::new();
        if let Some(m) = self.mappings.get(id.parse::<usize>().unwrap()) {
            names.extend(m.get_ordered_col_names());
        }
        names
    }

    pub fn scale_info(&self, scale : &str) -> HashMap<String, String> {
        match scale {
            "x" => self.x.description(),
            "y" => self.y.description(),
            _ => HashMap::new()
        }
    }

    pub fn update_mapping_columns(
        &mut self,
        id : &str,
        columns : Vec<String>
    ) -> Result<(), Box<dyn Error>> {
        if let Some(mapping) = self.mappings.get_mut(id.parse::<usize>().unwrap()) {
            if let Err(e) = mapping.set_col_names(columns) {
                println!("{}", e);
            }
        } else {
            println!("Mapping not found when updating column name");
        }
        if let Err(e) = self.reload_layout_node() {
            println!("{}", e);
        }
        Ok(())
    }

    pub fn update_source(
        &mut self,
        id : &str,
        source : String
    ) -> Result<(), Box<dyn Error>> {
        if let Some(mapping) = self.mappings.get_mut(id.parse::<usize>().unwrap()) {
            mapping.set_source(source);
        } else {
            println!("Mapping not found when updating column name");
        }
        Ok(())
    }

    /*pub fn update_mapping_column(
        &mut self,
        id : &str,
        column : &str,
        name : &str
    ) {
        if let Some(mapping) = self.mappings.get_mut(id.parse::<usize>().unwrap()) {
            mapping.set_col_name(column, name);
        } else {
            println!("Mapping not found when updating column name");
        }
        if let Err(e) = self.reload_layout_data() {
            println!("{}", e);
        }
    }*/

    /*pub fn get_mapping_column(
        &self,
        id : &str,
        column : &str
    ) -> Option<String> {
        if let Some(mapping) = self.mappings.get(id.parse::<usize>().unwrap()) {
            let col_name = mapping.get_col_name(column);
            if col_name != "None" {
                Some(col_name)
            } else {
                None
            }
        } else {
            println!("Mapping not found when getting column name");
            None
        }
    }*/

}

//#[repr(C)]

pub mod utils {

    use super::Node;
    use super::HashMap;
    use super::Document;
    use super::Error;

    /// Return all children of node that satisfy the
    /// informed xpath.
    pub fn children_as_hash(
        node : &Node,
        xpath : &str
    ) -> HashMap<String, String> {
        let mut prop_hash = HashMap::new();
        if let Ok(props) = node.findnodes(xpath) {
            if props.len() == 0 {
                panic!("No children found for node {:?} at path {}", node, xpath);
            }
            for prop in props.iter() {
                // println!("Property = {:?}", prop);
                let name = prop.get_attribute("name")
                    .expect(&format!("No name attribute found for property {:?}", prop));
                let value = prop.get_content();
                prop_hash.insert(name, value);
            }
        } else {
            panic!("Failed to retrieve children of {:?} at path {}", node, xpath);
        }
        prop_hash
    }

    pub fn populate_node_with_hash(
        doc : &Document,
        node : &mut Node,
        hash : HashMap<String, String>
    ) -> Result<(), Box<dyn Error>> {
        for (k, v) in hash {
            let mut property = Node::new(
                "property", Option::None, doc).unwrap();
            property.set_attribute("name", &k[..])?;
            property.set_content(&v[..])?;
            node.add_child(&mut property)?;
        }
        Ok(())
    }

}

//impl IsA<gtk::DrawingArea> for PlotView {
//}

//Draw
/*impl ObjectImpl for PlotView {

    glib_object_impl!();

    //fn get_type_data(&self) -> NonNull<TypeData> {
    //}

    //glib_wrapper! {
    //}
}*/
//impl AsRef
//unsafe impl IsA<gtk::DrawingArea> for PlotView {
//}
/*impl ObjectSubclass for PlotView {
    const NAME: &'static str = "PlotView";
    type ParentType = gtk::DrawingArea;
    /* Glib classes are global runtime structs that are created
    when the first object of a given class is instantiated,
    and are destroyed when the last object of a given class
    is destroyed. (There is only a single instance of each
    class at any given time). The alias "Class" automatically
    implements a boilerplate struct to hold this class. */
    type Class = subclass::simple::ClassStruct<Self>;
    /* The instante is a global runtime struct (also one for
    each registered object) that describes things like
    memory object layout. Also automatically created. */
    type Instance = subclass::simple::InstanceStruct<Self>;

    glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        let plot_area = PlotArea::new(String::from("assets/layout.xml"));
        PlotView{plot_area}
    }
}*/
// glib::Object::new(T::get_type(), &[])
// get_type() registers type
// glib_wrapper!

// Used for overriding virtual methods - Must map to
// Impl trait
//unsafe impl IsSubclassable<PlotView>
//for gtk::auto::drawing_area::DrawingAreaClass {

//}

//subclass::types::register_type();

/*impl ObjectSubclass for PlotView {
    const NAME: &'static str = "PlotView";

    type ParentType = gtk::DrawingArea;

    type Instance = PlotView;
    type Class = PlotViewClass;

    glib_object_subclass!();

    fn class_init(klass: &mut PlotView) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        PlotView::new();
    }
}*/
/*fn add_signal(
    &mut self,
    name: &str,
    flags: SignalFlags,
    arg_types: &[Type],
    ret_type: Type
)*/
//unsafe extern "C" fn

use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::{RefCell};
use std::fs::File;
//use gtkplotview::PlotView;
use std::io::Read;
use gtkplotview::plot_view::{PlotView, UpdateContent};
use gtkplotview::PlotArea;
use std::path::PathBuf;
//use mapping_menu::MappingMenu;
use super::layout_aux::*;
use crate::tables::{source::EnvironmentSource, environment::TableEnvironment};
//use tables::button::TableChooser;
use std::boxed;
//use layout_menu::*;
//use save_widgets::*;
use std::collections::HashMap;
use gdk::RGBA;
use gtk::prelude::*;

#[derive(Clone)]
pub struct DesignMenu {
    grid_thickness_scale : Scale,
    bg_color_btn : ColorButton,
    grid_color_btn : ColorButton,
    font_btn : FontButton
}

impl DesignMenu {

    pub fn update(&self, properties : HashMap<String, String>) {
        self.grid_thickness_scale.get_adjustment().set_value(properties["grid_width"].parse().unwrap());
        let bc : gdk::RGBA = properties["bg_color"].parse().unwrap();
        self.bg_color_btn.set_rgba(&bc);
        let gc : gdk::RGBA = properties["grid_color"].parse().unwrap();
        self.grid_color_btn.set_rgba(&gc);
        self.font_btn.set_font_name(&properties["font"]);
    }

}

pub fn build_design_menu(builder : &Builder, pl_view : Rc<RefCell<PlotView>>) -> DesignMenu {
    let grid_thickness_scale : Scale =
    builder.get_object("grid_thickness_scale").unwrap();
    let ref_view = pl_view.clone();
    connect_update_scale_property(&grid_thickness_scale,
        ref_view.clone(), "".to_string(), "grid_width".to_string());
    let bg_color_btn : ColorButton =
        builder.get_object("bg_color").unwrap();
    let grid_color_btn : ColorButton =
        builder.get_object("grid_color").unwrap();
    let ref_view = pl_view.clone();
    connect_update_color_property(&bg_color_btn,
        ref_view.clone(), "".to_string(), "bg_color".to_string());
    connect_update_color_property(&grid_color_btn,
        ref_view.clone(), "".to_string(), "grid_color".to_string());
    let font_btn : FontButton =
        builder.get_object("font_btn").unwrap();
    connect_update_font_property(&font_btn,
        ref_view.clone(), "".to_string(), "font".to_string());
    DesignMenu {
        grid_thickness_scale,
        bg_color_btn,
        grid_color_btn,
        font_btn
    }
}


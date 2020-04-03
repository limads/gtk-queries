use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;
use gtkplotview::plot_view::{PlotView, UpdateContent};

fn change_plot_property(
    plot_view : Rc<RefCell<PlotView>>,
    prefix : &str,
    name : &str,
    value : &str
) {
    let mut full_name = if prefix.is_empty() {
        "//property".to_owned()
    } else {
        "/gridplot/object[@name='".to_owned() + prefix + "']/property"
    };
    full_name = full_name + "[@name='" + name + "']";
    if let Ok(mut ref_view) = plot_view.try_borrow_mut() {
        ref_view.update(
            &mut UpdateContent::Layout(full_name, value.to_string()) );
        // ref_view.update_layout(&property_name, value);
        // ref_view.reload_layout_data().unwrap();
    }
}

pub fn connect_update_entry_property(
    entry : &Entry,
    view : Rc<RefCell<PlotView>>,
    prefix : String,
    name : String) {
    entry.connect_focus_out_event(move |entry, _ev| {
        if let Some(txt) = entry.get_text() {
            if txt.len() > 0 {
                change_plot_property(view.clone(), &prefix, &name, txt.as_str());
            }
        }
        Inhibit(false)
    });
}

pub fn connect_update_switch_property(
    switch : &Switch,
    view : Rc<RefCell<PlotView>>,
    prefix : String,
    name : String) {
    switch.connect_state_set(move |_switch, state| {
        change_plot_property(view.clone(), &prefix, &name, &state.to_string());
        Inhibit(false)
    });
}

pub fn connect_update_scale_property(
    scale : &Scale,
    view : Rc<RefCell<PlotView>>,
    prefix : String,
    name : String
) {
    let scale_fn = move |adj : &Adjustment| {
        let val = adj.get_value();
        change_plot_property(view.clone(), &prefix, &name, &val.to_string());
    };

    scale.get_adjustment().connect_value_changed(scale_fn.clone());
    scale.get_adjustment().connect_changed(scale_fn);
}

pub fn connect_update_color_property(
    btn : &ColorButton,
    view : Rc<RefCell<PlotView>>,
    prefix : String,
    name : String) {
    btn.connect_color_set( move |btn| {
        change_plot_property(view.clone(), &prefix, &name,
            &btn.get_rgba().to_string());
    });
}

pub fn connect_update_font_property(
    btn : &FontButton,
    view : Rc<RefCell<PlotView>>,
    prefix : String,
    name : String) {
    btn.connect_font_set( move |btn| {
        if let Some(font) = btn.get_font() {
            change_plot_property(view.clone(), &prefix,
                &name, font.as_str());
        }
    });
}



use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
// use gtk::prelude::*;
use super::plotview::plot_view::{PlotView, UpdateContent};

fn change_plot_property(
    plot_view : Rc<RefCell<PlotView>>,
    prefix : &str,
    name : &str,
    parent_class : &str,
    value : &str
) {
    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
        let mut full_name = format!("/plotgroup/plotarea[@position={}]", pl_view.get_active_area());
        let identifier = match parent_class {
            "mapping" => "@index",
            "grid_segment" | "design" => "@name",
            _ => { println!("Invalid parent class: {}", parent_class); return; }
        };
        match parent_class {
            "mapping" | "grid_segment" | "design" => {
                full_name += &("/object[".to_owned() + identifier + "='" + prefix + "']/property")[..];
            },
            _ => {
                full_name += "/property";
            }
        }
        full_name += &("[".to_owned() + "@name" + "='" + name + "']")[..];
        println!("{}", full_name);
        pl_view.update(&mut UpdateContent::Layout(full_name, value.to_string()) );
    } else {
        println!("Unable to get mutable reference to plot view");
    }
}

pub fn connect_update_entry_property(
    entry : &Entry,
    view : Rc<RefCell<PlotView>>,
    prefix : Rc<RefCell<String>>,
    name : String,
    parent_class : &'static str
) {
    entry.connect_focus_out_event(move |entry, _ev| {
        if let Ok(prefix) = prefix.try_borrow() {
            if let Some(txt) = entry.get_text() {
                if txt.len() > 0 {
                    change_plot_property(
                        view.clone(),
                        &prefix[..],
                        &name,
                        parent_class,
                        txt.as_str()
                    );
                }
            }
        } else {
            println!("Unable to retrieve reference to mapping name");
        }
        Inhibit(false)
    });
}

pub fn connect_update_switch_property(
    switch : &Switch,
    view : Rc<RefCell<PlotView>>,
    prefix : Rc<RefCell<String>>,
    name : String,
    parent_class : &'static str
) {
    switch.connect_state_set(move |_switch, state| {
        if let Ok(prefix) = prefix.try_borrow() {
            change_plot_property(
                view.clone(),
                &prefix[..],
                &name,
                parent_class,
                &state.to_string()
            );
        } else {
            println!("Unable to retrieve reference to mapping name");
        }
        Inhibit(false)
    });
}

pub fn connect_update_scale_property(
    scale : &Scale,
    view : Rc<RefCell<PlotView>>,
    prefix : Rc<RefCell<String>>,
    name : String,
    parent_class : &'static str
) {
    let scale_fn = move |adj : &Adjustment| {
        if let Ok(prefix) = prefix.try_borrow() {
            let val = adj.get_value();
            change_plot_property(
                view.clone(),
                &prefix[..],
                &name,
                parent_class,
                &val.to_string()
            );
        } else {
            println!("Unable to retrieve reference to mapping name");
        }
    };
    scale.get_adjustment().connect_value_changed(scale_fn.clone());
    scale.get_adjustment().connect_changed(scale_fn);
}

pub fn connect_update_color_property(
    btn : &ColorButton,
    view : Rc<RefCell<PlotView>>,
    prefix : Rc<RefCell<String>>,
    name : String,
    parent_class : &'static str
) {
    btn.connect_color_set( move |btn| {
        if let Ok(prefix) = prefix.try_borrow() {
            change_plot_property(
                view.clone(),
                &prefix[..],
                &name,
                parent_class,
                &btn.get_rgba().to_string()
            );
        } else {
            println!("Unable to retrieve reference to mapping name");
        }
    });
}

pub fn connect_update_font_property(
    btn : &FontButton,
    view : Rc<RefCell<PlotView>>,
    prefix : Rc<RefCell<String>>,
    name : String,
    parent_class : &'static str
) {
    btn.connect_font_set( move |btn| {
        if let Ok(prefix) = prefix.try_borrow() {
            if let Some(font) = btn.get_font() {
                change_plot_property(
                    view.clone(),
                    &prefix[..],
                    &name,
                    parent_class,
                    font.as_str()
                );
            }
        } else {
            println!("Unable to retrieve reference to mapping name");
        }
    });
}



use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;
use gtkplotview::plot_view::{PlotView, UpdateContent};

fn change_plot_property(
    plot_view : Rc<RefCell<PlotView>>,
    prefix : &str,
    name : &str,
    parent_class : &str,
    value : &str
) {
    let identifier = match parent_class {
        "mapping" => "@index",
        "grid_segment" | "design" => "@name",
        _ => { println!("Invalid parent class: {}", parent_class); return; }
    };
    let mut full_name = if prefix.is_empty() {
        "//property".to_owned()
    } else {
        "/gridplot/object[".to_owned() + identifier + "='" + prefix + "']/property"
    };
    full_name = full_name + "[" + "@name" + "='" + name + "']";
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



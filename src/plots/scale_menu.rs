use gtk::*;
// use gio::prelude::*;
// use crate::*;
use std::collections::HashMap;
use gtk::prelude::*;
use crate::plots::plotview::plot_view::PlotView;
// use crate::plots::plotview::PlotArea;
use super::layout_aux::*;
use std::rc::Rc;
use std::cell::RefCell;
use glib::SignalHandlerId;

#[derive(Clone, Debug)]
pub struct ScaleMenu {
    label_entry : Entry,
    entry_min : Entry,
    entry_max : Entry,
    log_switch : Switch,
    invert_switch : Switch,
    offset_scale : Scale,
    density_scale : Scale,
    signals : Rc<RefCell<Vec<SignalHandlerId>>>
}

impl ScaleMenu {

    pub fn update(&self, properties : HashMap<String, String>) {
        let signals = self.signals.borrow();
        
        self.label_entry.block_signal(&signals[0]);
        self.entry_min.block_signal(&signals[1]);
        self.entry_max.block_signal(&signals[2]);
        self.log_switch.block_signal(&signals[3]);
        self.invert_switch.block_signal(&signals[4]);
        self.offset_scale.get_adjustment().block_signal(&signals[5]);
        self.density_scale.get_adjustment().block_signal(&signals[6]);
        
        self.label_entry.set_text(&properties["label"]);
        self.entry_min.set_text(&properties["from"]);
        self.entry_max.set_text(&properties["to"]);
        self.log_switch.set_state(properties["log_scaling"].parse().unwrap());
        self.invert_switch.set_state(properties["invert"].parse().unwrap());
        self.offset_scale.get_adjustment().set_value(properties["grid_offset"].parse().unwrap());
        self.density_scale.get_adjustment().set_value(properties["n_intervals"].parse().unwrap());
        
        self.label_entry.unblock_signal(&signals[0]);
        self.entry_min.unblock_signal(&signals[1]);
        self.entry_max.unblock_signal(&signals[2]);
        self.log_switch.unblock_signal(&signals[3]);
        self.invert_switch.unblock_signal(&signals[4]);
        self.offset_scale.get_adjustment().unblock_signal(&signals[5]);
        self.density_scale.get_adjustment().unblock_signal(&signals[6]);
    }

}

/// suffix must be either 'x' or 'y'
/// (name of the grid_segment object)
pub fn prepare_scale_menu(
    builder : &Builder,
    view : Rc<RefCell<PlotView>>,
    suffix : &str
) -> ScaleMenu {
    let mut signals = Vec::new();
    let label_entry : Entry = builder.get_object(
        &("label_entry_".to_string() + suffix)).unwrap();
    signals.push(connect_update_entry_property(
        &label_entry,
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "label".to_string(),
        "grid_segment"
    ));
    let entry_min : Entry = builder.get_object(
        &("entry_min_".to_string() + suffix)).unwrap();
    let entry_max : Entry = builder.get_object(
        &("entry_max_".to_string() + suffix)).unwrap();
    signals.push(connect_update_entry_property(
        &entry_min,
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "from".to_string(),
        "grid_segment"
    ));
    signals.push(connect_update_entry_property(
        &entry_max, 
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "to".to_string(),
        "grid_segment"
    ));
    let log_switch : Switch = builder.get_object(
        &("log_switch_".to_string() + suffix)).unwrap();
    let invert_switch : Switch = builder.get_object(
        &("invert_switch_".to_string() + suffix)).unwrap();
    signals.push(connect_update_switch_property(
        &log_switch,
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "log_scaling".to_string(),
        "grid_segment"
    ));
    {
        let invert_switch = invert_switch.clone();
        let view = view.clone();
        let entry_min = entry_min.clone();
        log_switch.connect_state_set(move |_switch, state| {
            if state {
                /*if let Some(txt) = entry_min.get_text() {
                    if let Ok(val) = txt.parse::<f64>() {
                        if val <= 0.0 {
                            entry_min.set_text("0.001");
                        }
                    }
                }*/
                if invert_switch.get_active() {
                    // invert_switch.set_active(false);
                    // pl_view.update(&mut UpdateContent::Layout(format!("log_switch_{}", prefix), "false") );
                }
            }
            glib::signal::Inhibit(true)
        });
    }
    signals.push(connect_update_switch_property(
        &invert_switch,
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "invert".to_string(),
        "grid_segment"
    ));
    let offset_scale : Scale = builder.get_object(
        &("grid_offset_".to_string() + suffix)).unwrap();
    let density_scale : Scale = builder.get_object(
        &("grid_density_".to_string() + suffix)).unwrap();
    signals.push(connect_update_scale_property(
        &offset_scale,
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "grid_offset".to_string(),
        "grid_segment"
    ));
    signals.push(connect_update_scale_property(
        &density_scale,
        view.clone(),
        Rc::new(RefCell::new(suffix.to_string())),
        "n_intervals".to_string(),
        "grid_segment"
    ));
    ScaleMenu {
        label_entry,
        entry_min,
        entry_max,
        log_switch,
        invert_switch,
        offset_scale,
        density_scale,
        signals : Rc::new(RefCell::new(signals))
    }
}

pub fn build_scale_menus(
    builder : &Builder,
    view : Rc<RefCell<PlotView>>
) -> (ScaleMenu, ScaleMenu) {
    /*let scales_box : Box =
        builder.get_object("scales_box").unwrap();
    let expander : Expander =
        builder.get_object("scales_expander").unwrap();
    expander.add(&scales_box);*/

    //let box_x : Box = builder.get_object("scale_box_x").unwrap();
    //let box_y : Box = builder.get_object("scale_box_y").unwrap();
    /*box_y.hide();
    box_x.hide();
    let toggle_x : ToggleButton =
        builder.get_object("toggle_scale_x").unwrap();
    let toggle_y : ToggleButton =
        builder.get_object("toggle_scale_y").unwrap();
    toggle_x.set_active(false);
    toggle_y.set_active(false);

    {
        let box_x = box_x.clone();
        let box_y = box_y.clone();
        let toggle_y = toggle_y.clone();
        toggle_x.connect_toggled(move |_| {
            //box_y.hide();
            //box_x.show();
            toggle_y.set_active(false);
            //toggle_y.toggled();
        });
    }

    {
        let box_x = box_x.clone();
        let box_y = box_y.clone();
        let toggle_x = toggle_x.clone();
        toggle_y.connect_toggled(move |_| {
            //box_x.hide();
            //box_y.show();
            toggle_x.set_active(false);
            //toggle_x.toggled();
        });
    }*/

    let scale_menu_x =
        prepare_scale_menu(builder, view.clone(), "x");
    let scale_menu_y =
        prepare_scale_menu(builder, view.clone(), "y");
    (scale_menu_x, scale_menu_y)
}


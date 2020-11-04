use gtk::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use gtk::prelude::*;
use crate::table_notebook::TableNotebook;

#[derive(Clone)]
pub struct UploadPopover {
    toggle : ToggleButton,
    popover : Popover,
    chooser : FileChooserButton,
    switch : Switch,

    // Update from columns indexed by the first element; using SQL template from second element.
    update_info : Rc<RefCell<(Vec<usize>, String)>>
}

impl UploadPopover {

    pub fn new(builder : Builder, tbl_nb : TableNotebook) -> Self {
        let toggle : ToggleButton = builder.get_object("upload_toggle").unwrap();
        let popover : Popover = builder.get_object("upload_popover").unwrap();
        let chooser : FileChooserButton = builder.get_object("sql_upload_chooser").unwrap();
        let switch : Switch = builder.get_object("sql_upload_switch").unwrap();
        {
            let popover = popover.clone();
            toggle.connect_toggled(move |toggle| {
                if toggle.get_active() {
                    popover.show();
                } else {
                    popover.hide();
                }
            });
        }
        let update_info = Rc::new(RefCell::new((Vec::new(), String::new())));
        {
            let update_info = update_info.clone();
            chooser.connect_file_set(move |chooser| {
                if let Some(path) = chooser.get_filename() {
                    if let Ok(mut f) = File::open(path) {
                        if let Ok(mut info) = update_info.try_borrow_mut() {
                            if let Err(e) = f.read_to_string(&mut info.1) {
                                println!("{}", e);
                            }
                        } else {
                            println!("Could not retrieve mutable reference to sql upload file");
                        }
                    } else {
                        println!("Error opening file");
                    }
                }
            });
        }

        {
            let update_info = update_info.clone();
            switch.connect_state_set(move |_switch, _state| {
                let selected = tbl_nb.full_selected_cols();
                if let Ok(mut info) = update_info.try_borrow_mut() {
                    info.0.clear();
                    info.0.extend(selected);
                } else {
                    println!("Unable to get mutable reference to update info");
                }
                glib::signal::Inhibit(true)
            });
        }

        Self{ toggle, popover, chooser, switch, update_info }
    }

}


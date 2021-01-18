use gtk::*;
use gtk::prelude::*;
use gio::prelude::*;
use std::env::args;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use crate::utils;

pub struct JobLoader {

}

pub struct JobManager {
    job_arg_btn : ToggleButton,
    job_popover : Popover,
    job_list_box : ListBox,
    job_doc_btn : ToggleButton,
    doc_popover : Popover,
    // job_run_btn : ToggleButton,
    job_add_btn : Button,
    job_rm_btn : Button
}

impl JobManager {

    fn build_job_item(name : &str) -> ListBoxRow {
        let bx = Box::new(Orientation::Horizontal, 0);
        // let check = CheckButton::new();
        // check.set_active(true);
        let name_string = name.to_string();
        /*check.connect_toggled(move |btn| {
            /*if let Ok(mut loader) = loader.lock() {
                if let Err(e) = loader.set_active_status(&name_string[..], btn.get_active()) {
                    println!("{}", e);
                }
            } else {
                println!("Not possible to lock loader");
            }*/
        });*/

        let lbl_name = Label::new(Some(name));
        let icon_error = Image::from_icon_name(Some("application-exit"), IconSize::SmallToolbar);
        let icon_ok = Image::from_icon_name(Some("object-select-symbolic"), IconSize::SmallToolbar);
        let bx_child = Box::new(Orientation::Horizontal, 0);
        bx_child.pack_start(&icon_ok, false, false, 0);
        bx_child.pack_start(&lbl_name, false, false, 0);
        bx_child.set_property_width_request(120);

        let lbl_times = Label::new(Some("0"));
        lbl_times.set_property_width_request(120);
        let lbl_duration = Label::new(Some("0:23"));
        lbl_duration.set_property_width_request(120);
        // lbl_duration.set_margin_start(60);
        // check.set_label(Some(name));

        bx.pack_start(&bx_child, true, true, 0);
        bx.pack_start(&lbl_times, true, true, 0);
        bx.pack_start(&lbl_duration, true, true, 0);
        let row = ListBoxRow::new();
        row.add(&bx);
        row.set_selectable(true);
        row.set_property_height_request(24);
        row
    }

    fn append_job(&self) {
        self.job_list_box.insert(&Self::build_job_item("analyze_data"), 0 as i32);
        self.job_list_box.show_all();
    }

    pub fn build(builder : &Builder) -> Self {
        let job_arg_btn : ToggleButton = builder.get_object("job_arg_btn").unwrap();
        let job_popover : Popover = builder.get_object("job_popover").unwrap();
        let doc_popover : Popover = builder.get_object("job_doc_popover").unwrap();
        let job_doc_btn : ToggleButton = builder.get_object("job_doc_btn").unwrap();
        let job_list_box : ListBox = builder.get_object("job_list_box").unwrap();
        let job_arg_btn : ToggleButton = builder.get_object("job_arg_btn").unwrap();
        // let job_run_btn : ToggleButton = builder.get_object("job_run_btn").unwrap();
        let job_add_btn : Button = builder.get_object("job_add_btn").unwrap();
        let job_rm_btn : Button = builder.get_object("job_rm_btn").unwrap();
        utils::show_popover_on_toggle(&job_popover, &job_arg_btn, vec![]);
        utils::show_popover_on_toggle(&doc_popover, &job_doc_btn, vec![]);
        let manager = Self {
            job_arg_btn,
            job_popover,
            job_list_box,
            doc_popover,
            job_doc_btn,
            // job_run_btn,
            job_add_btn,
            job_rm_btn
        };
        manager.append_job();
        manager
    }

}

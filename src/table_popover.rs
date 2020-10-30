use gtk::*;
use gio::prelude::*;
// use std::env::args;
// use std::rc::Rc;
// use std::cell::RefCell;
// use std::fs::File;
// use std::io::Write;
use gdk::{self, keys};
use sourceview::*;
use gtk::prelude::*;
// use crate::{utils, table_notebook::TableNotebook };

#[derive(Clone)]
pub struct TablePopover {
    pub popover : Popover,
    // pub command_box : Box,
    //pub copy_box : Box,
    pub upload_box : Box,
    // pub command_btn : Button,
    // pub upload_btn : Button,
    finish_upload_btn : Button,
    apply_btn : Button,
    clear_btn : Button,
    command_entry : Entry,
    table_size_label : Label,
    table_latency_label : Label,
    backward_btn : Button,
    forward_btn : Button,
}

impl TablePopover {

    pub fn build(builder :  &Builder) -> Self {
        let popover : Popover = builder.get_object("table_popover").unwrap();
        // let command_box : Box = builder.get_object("command_box").unwrap();
        //let copy_box : Box = builder.get_object("table_copy_box").unwrap();
        let upload_box : Box = builder.get_object("table_upload_box").unwrap();

        let table_size_label : Label = builder.get_object("table_size_label").unwrap();
        let table_latency_label : Label = builder.get_object("table_latency_label").unwrap();
        let forward_btn : Button = builder.get_object("table_forward_btn").unwrap();
        let backward_btn : Button = builder.get_object("table_backward_btn").unwrap();

        //let upload_btn : Button = builder.get_object("upload_button").unwrap();
        let finish_upload_btn : Button = builder.get_object("finish_upload_button").unwrap();

        let apply_btn : Button = builder.get_object("command_apply_btn").unwrap();
        let clear_btn : Button = builder.get_object("command_clear_btn").unwrap();
        let command_entry : Entry = builder.get_object("command_entry").unwrap();
        Self {
            popover,
            // command_box,
            // command_btn,
            apply_btn,
            clear_btn,
            command_entry,
            // copy_box,
            upload_box,
            table_size_label,
            table_latency_label,
            // upload_btn,
            finish_upload_btn,
            forward_btn,
            backward_btn
        }
    }

}



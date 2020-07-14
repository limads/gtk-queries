use gtk::*;
use gio::prelude::*;
use std::rc::Rc;
use std::cell::{RefCell /*, RefMut*/ };
use std::fs::File;
use std::io::Read;
use gdk::{self, enums::key};
use crate::tables::{ /*self, source::EnvironmentSource,*/ environment::TableEnvironment, environment::EnvironmentUpdate, /*sql::SqlListener*/ };
use sourceview::*;
use gtk::prelude::*;
use crate::{ /*utils, table_widget::TableWidget, table_notebook::TableNotebook,*/ status_stack::StatusStack };
use crate::status_stack::*;
use sourceview::View;

#[derive(Clone, Debug)]
pub struct MainMenu {
    main_menu : PopoverMenu,
    main_toggle : ToggleButton,
    local_funcs_btn : ModelButton,
    funcs_window : Window,
    library_list_box : ListBox,
    fn_list_box : ListBox
}

impl MainMenu {

    pub fn new(builder : &Builder) -> Self {
        let main_menu : PopoverMenu = builder.get_object("main_menu").unwrap();
        let main_toggle : ToggleButton = builder.get_object("main_toggle").unwrap();
        let local_funcs_btn : ModelButton = builder.get_object("local_funcs_btn").unwrap();
        let funcs_window : Window = builder.get_object("funcs_window").unwrap();
        let library_list_box : ListBox = builder.get_object("library_list_box").unwrap();
        let fn_list_box : ListBox = builder.get_object("fn_list_box").unwrap();
        {
            let main_menu = main_menu.clone();
            main_toggle.connect_toggled(move |btn| {
                if btn.get_active() {
                    main_menu.show();
                } else {
                    main_menu.hide();
                }
            });
        }

        {
            let main_toggle = main_toggle.clone();
            main_menu.connect_closed(move |_| {
                if main_toggle.get_active() {
                    main_toggle.set_active(false);
                }
            });
        }

        {
            let funcs_window = funcs_window.clone();
            local_funcs_btn.connect_clicked(move |_| {
                funcs_window.show();
            });
        }

        MainMenu { main_menu, main_toggle, local_funcs_btn, funcs_window, library_list_box, fn_list_box }
    }

}



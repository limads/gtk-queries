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
    engine_btn : ModelButton,
    engine_window : Window,
    settings_btn : ModelButton,
    settings_window : Window,
    layout_window : Window,
    layout_btn : ModelButton
    // library_list_box : ListBox,
    // fn_list_box : ListBox
}

impl MainMenu {

    fn link_window(btn : ModelButton, win : Window) {
        {
        let win = win.clone();
            btn.connect_clicked(move |_| {
                win.show();
            });
        }
        win.set_destroy_with_parent(false);
        win.connect_delete_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });
        win.connect_destroy_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });
    }

    pub fn new(builder : &Builder) -> Self {
        let main_menu : PopoverMenu = builder.get_object("main_menu").unwrap();
        let main_toggle : ToggleButton = builder.get_object("main_toggle").unwrap();
        let engine_btn : ModelButton = builder.get_object("engine_btn").unwrap();
        let layout_btn : ModelButton = builder.get_object("layout_btn").unwrap();
        let settings_btn : ModelButton = builder.get_object("settings_btn").unwrap();
        let engine_window : Window = builder.get_object("engine_window").unwrap();
        let settings_window : Window = builder.get_object("settings_window").unwrap();
        let layout_window : Window = builder.get_object("layout_window").unwrap();
        // engine_window.set_destroy_with_parent(false);
        // settings_window.set_destroy_with_parent(false);
        Self::link_window(engine_btn.clone(), engine_window.clone());
        Self::link_window(settings_btn.clone(), settings_window.clone());
        Self::link_window(layout_btn.clone(), layout_window.clone());
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



        /*{
            let settings_window = settings_window.clone();
            settings_btn.connect_clicked(move |_| {
                settings_window.show();
                //funcs_window.hide();
            });
        }*/
        /*engine_window.connect_delete_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });

        engine_window.connect_destroy_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });

        settings_window.connect_delete_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });

        settings_window.connect_destroy_event(move |win, ev| {
            win.hide();
            glib::signal::Inhibit(true)
        });*/

        MainMenu { main_menu, main_toggle, engine_btn, engine_window, settings_btn, settings_window, layout_window, layout_btn }
    }

}



use gtk::*;
use gio::prelude::*;
use std::env::args;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use gdk::{self, keys};
use gtk_queries::tables::{source::EnvironmentSource, environment::TableEnvironment, environment::EnvironmentUpdate};
use gtk_queries::conn_popover::*;
use sourceview::*;
use gtk::prelude::*;
use gtk_queries::{utils, table_notebook::TableNotebook };
use gtk_queries::status_stack::*;
use gtk_queries::sql_editor::*;
use gtk_queries::functions::registry::FunctionRegistry;
use gtk_queries::plots::plotview::plot_view::PlotView;
use gtk_queries::plots::plot_workspace::PlotWorkspace;
use gtk_queries::query_sidebar::QuerySidebar;
use gtk_queries::main_menu::MainMenu;
use gtk_queries::plots::layout_toolbar::*;
use gtk_queries::file_list::FileList;
use gtk_queries::schema_tree::SchemaTree;
use gtk_queries::jobs::JobManager;

#[derive(Clone)]
pub struct QueriesApp {
    conn_popover : ConnPopover,
    sql_editor : SqlEditor,
    table_env : Rc<RefCell<TableEnvironment>>,
    tables_nb : TableNotebook,
    main_paned : Paned,
    table_toggle : ToggleButton,
    plot_toggle : ToggleButton,
    query_toggle : ToggleButton,
    paned_pos : Rc<RefCell<i32>>,
    main_menu : MainMenu,
    plot_workspace : PlotWorkspace,
    schema_tree : SchemaTree
}

/*fn adjust_sidebar_pos(btn : &ToggleButton, window : &Window, main_paned : &Paned) {
    if let Some(win) = window.get_window() {
        let w = win.get_width();
        match btn.get_active() {
            false => {
                main_paned.set_position(w);
            },
            true => {
                let adj_w = (w as f32 * 0.8) as i32;
                main_paned.set_position(adj_w);
            }
        }
    }
}*/

impl QueriesApp {

    fn build_plots_widgets(
        builder : Builder,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        pl_da : DrawingArea,
        plot_toggle : ToggleButton,
        table_toggle : ToggleButton,
        status_stack : StatusStack,
        sidebar_stack : Stack,
    ) -> PlotWorkspace {
        //let builder = Builder::new_from_file(utils::glade_path("gtk-plots-stack.glade").unwrap());
        let pl_view = PlotView::new_with_draw_area(
            "assets/plot_layout/layout-unique.xml", pl_da.clone());
        // save_widgets::build_save_widgets(&builder, pl_view.clone());
        let workspace = PlotWorkspace::new(
            builder.clone(),
            pl_view.clone(),
            table_env.clone(),
            tbl_nb.clone(),
            status_stack,
            plot_toggle.clone(),
            table_toggle.clone(),
            sidebar_stack.clone()
        );
        //sidebar_stack.add_named(&sidebar.layout_stack, "layout");
        workspace
    }

    fn switch_paned_pos(main_paned : Paned, paned_pos : Rc<RefCell<i32>>, state : bool) {
        if state {
            if let Ok(s_pos) = paned_pos.try_borrow() {
                main_paned.set_position(*s_pos);
            } else {
                println!("Unable to retrieve sidebar position");
            }
        } else {
            if let Ok(mut s_pos) = paned_pos.try_borrow_mut() {
                *s_pos = main_paned.get_position();
                if main_paned.get_orientation() == Orientation::Horizontal {
                    main_paned.set_position(0);
                } else {
                    //println!("{:?}", main_paned.get_allocation());
                    main_paned.set_position(main_paned.get_allocation().height);
                }
            } else {
                println!("Unable to retrieve sidebar position");
            }
        }
    }

    fn connect_toggles(
        paned_pos : Rc<RefCell<i32>>,
        query_toggle : ToggleButton,
        table_toggle : ToggleButton,
        plot_toggle : ToggleButton,
        _builder : Builder,
        main_paned : Paned,
        sidebar_stack : Stack,
        content_stack : Stack,
        status_stack : StatusStack,
        workspace : PlotWorkspace,
        mapping_popover : Popover,
        file_list : FileList
    ) {
        {
            let main_paned_c = main_paned.clone();
            let paned_pos = paned_pos.clone();
            main_paned.connect_size_allocate(move |_paned_wid, _all| {
                if let Ok(mut s_pos) = paned_pos.try_borrow_mut() {
                    let new_pos = main_paned_c.get_position();
                    if new_pos > 0 {
                        *s_pos = new_pos
                    }
                } else {
                    println!("Error acquiring reference to main paned");
                }
            });
        }

        {
            let main_paned = main_paned.clone();
            let plot_toggle = plot_toggle.clone();
            let content_stack = content_stack.clone();
            let workspace = workspace.clone();
            let status_stack = status_stack.clone();
            let paned_pos = paned_pos.clone();
            let query_toggle = query_toggle.clone();
            let sidebar_stack = sidebar_stack.clone();
            table_toggle.connect_toggled(move |btn| {
                match btn.get_active() {
                    false => {
                        if !workspace.layout_loaded() {
                            status_stack.try_show_alt_or_connected();
                        }
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), false);
                        if plot_toggle.get_active() {
                            plot_toggle.toggled();
                            // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                        }
                        if query_toggle.get_active() {
                            query_toggle.toggled();
                        }

                    },
                    true => {
                        content_stack.set_visible_child_name("tables");
                        status_stack.try_show_alt_or_connected();
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                        if plot_toggle.get_active() {
                            plot_toggle.set_active(false);
                        }
                        if query_toggle.get_active() {
                            query_toggle.set_active(false);
                        }
                    }
                }
            });
        }

        {
            let main_paned = main_paned.clone();
            let table_toggle = table_toggle.clone();
            let paned_pos = paned_pos.clone();
            let content_stack = content_stack.clone();
            let query_toggle = query_toggle.clone();
            let status_stack = status_stack.clone();
            let sidebar_stack = sidebar_stack.clone();
            let workspace = workspace.clone();
            plot_toggle.connect_toggled(move |btn| {
                match btn.get_active() {
                    false => {
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), false);
                        if table_toggle.get_active() {
                            table_toggle.toggled();
                            // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                        }
                        if query_toggle.get_active() {
                            query_toggle.toggled();
                        }
                    },
                    true => {
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                        content_stack.set_visible_child_name("plot");
                        if let Ok(pl_view) = workspace.pl_view.try_borrow() {
                            pl_view.redraw();
                        } else {
                            println!("Failed to acquire lock over plot")
                        }
                        if table_toggle.get_active() {
                            table_toggle.set_active(false);
                        }
                        if query_toggle.get_active() {
                            query_toggle.set_active(false);
                        }
                        status_stack.try_show_alt();
                    }
                }
            });
        }

        {
            let plot_toggle = plot_toggle.clone();
            let table_toggle = table_toggle.clone();
            let status_stack = status_stack.clone();
            let paned_pos = paned_pos.clone();
            query_toggle.connect_toggled(move |btn| {
                match btn.get_active() {
                    false => {
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), false);
                        if plot_toggle.get_active() {
                            plot_toggle.toggled();
                        }
                        if table_toggle.get_active() {
                            table_toggle.toggled();
                        }
                    },
                    true => {
                        let page_name = if let Some(ix) = file_list.get_selected() {
                            format!("queries_{}", ix)
                        } else {
                            format!("no_queries")
                        };
                        println!("Setting visible: {}", page_name);
                        content_stack.set_visible_child_name(&page_name);
                        println!("Visible name: {:?}", content_stack.get_visible_child_name());
                        status_stack.show_alt();
                        // Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), true);
                        if table_toggle.get_active() {
                            table_toggle.set_active(false);
                        }
                        if plot_toggle.get_active() {
                            plot_toggle.set_active(false);
                        }
                    }
                }
            });
        }
    }

    pub fn new_from_builder(builder : &Builder, _window : Window, full : bool) -> Self {
        let main_paned : Paned =  builder.get_object("main_paned").unwrap();
        let paned_pos : Rc<RefCell<i32>> = Rc::new(RefCell::new(main_paned.get_position()));
        let sidebar_toggle : ToggleButton = builder.get_object("sidebar_toggle").unwrap();
        {
            let main_paned = main_paned.clone();
            let paned_pos = paned_pos.clone();
            sidebar_toggle.connect_toggled(move |btn| {
                Self::switch_paned_pos(main_paned.clone(), paned_pos.clone(), btn.get_active());
            });
        }
        let tables_nb = TableNotebook::new(&builder);

        let main_stack : Stack = builder.get_object("main_stack").unwrap();
        let sidebar_stack : Stack = builder.get_object("sidebar_stack").unwrap();
        let content_stack : Stack = builder.get_object("content_stack").unwrap();
        let status_stack = StatusStack::new(builder.clone(), main_stack, content_stack.clone().upcast::<Widget>());

        let table_toggle : ToggleButton = builder.get_object("table_toggle").unwrap();
        let plot_toggle : ToggleButton = builder.get_object("plot_toggle").unwrap();
        let query_toggle : ToggleButton = builder.get_object("query_toggle").unwrap();
        let (fn_reg, fn_loader) = FunctionRegistry::new(
            builder
        );
        // let job_manager = JobManager::build(&builder);

        let env_source = EnvironmentSource::File("".into(),"".into());
        let table_env = Rc::new(RefCell::new(TableEnvironment::new(env_source, fn_loader)));
        let conn_btn : Button = builder.get_object("conn_btn").unwrap();
        let popover_path = utils::glade_path("conn-popover.glade")
            .expect("Could not open glade path");
        let schema_tree = SchemaTree::build(&builder);
        let conn_popover = ConnPopover::new_from_glade(
            builder.clone(),
            conn_btn
        );
        let pl_da : DrawingArea = builder.get_object("plot").unwrap();
        let plot_workspace = Self::build_plots_widgets(
            builder.clone(),
            table_env.clone(),
            tables_nb.clone(),
            pl_da.clone(),
            plot_toggle.clone(),
            table_toggle.clone(),
            status_stack.clone(),
            sidebar_stack.clone()
        );

        let file_list = FileList::build(&builder);
        let mut sql_editor = SqlEditor::build(
            builder.clone(),
            table_toggle.clone(),
            query_toggle.clone(),
            status_stack.clone(),
            content_stack.clone(),
            table_env.clone(),
            &file_list
        );
        file_list.add_file_row(
            "Untitled 1",
            content_stack.clone(),
            sql_editor.clone(),
        );
        file_list.connect_selected(&sql_editor, content_stack.clone(), query_toggle.clone());
        conn_popover.hook_signals(
            table_env.clone(),
            tables_nb.clone(),
            status_stack.clone(),
            sql_editor.clone(),
            plot_workspace.clone(),
            fn_reg.clone(),
            schema_tree.clone()
        );

        {
            let tables_nb = tables_nb.clone();
            let status_stack = status_stack.clone();
            let query_toggle = query_toggle.clone();
            let plot_toggle = plot_toggle.clone();
            let table_toggle = table_toggle.clone();
            let file_list = file_list.clone();
            sql_editor.connect_send_query(move |res| {
                match res {
                    Ok(_) => {
                        tables_nb.nb.set_sensitive(false);
                        file_list.set_sensitive(false);
                    },
                    Err(e) => {
                        status_stack.update(Status::SqlErr(e));
                        table_toggle.set_active(true);
                    }
                }
                Ok(())
            });
        }

        let fn_toggle = ToggleToolButton::new();
        let fn_img = Image::from_file("assets/icons/fn-dark.svg");
        fn_toggle.set_icon_widget(Some(&fn_img));
        Self::connect_toggles(
            paned_pos.clone(),
            query_toggle.clone(),
            table_toggle.clone(),
            plot_toggle.clone(),
            builder.clone(),
            main_paned.clone(),
            sidebar_stack.clone(),
            content_stack.clone(),
            status_stack.clone(),
            plot_workspace.clone(),
            plot_workspace.layout_toolbar.mapping_popover.clone(),
            file_list.clone()
        );

        {
            let tables_nb = tables_nb.clone();
            let fn_reg = fn_reg.clone();
            let workspace = plot_workspace.clone();
            let status_stack = status_stack.clone();
            let query_toggle = query_toggle.clone();
            let plot_toggle = plot_toggle.clone();
            let table_toggle = table_toggle.clone();
            let mapping_popover = workspace.layout_toolbar.mapping_popover.clone();
            let file_list = file_list.clone();
            let f = move |t_env : &TableEnvironment, update : &EnvironmentUpdate| {
                match update {
                    EnvironmentUpdate::Clear => {
                        tables_nb.clear();
                        workspace.clear_mappings()
                            .map_err(|e| println!("{}", e) ).ok();
                    },
                    EnvironmentUpdate::NewTables(_) => {
                        utils::set_tables(
                            &t_env,
                            &mut tables_nb.clone(),
                            mapping_popover.clone(),
                            workspace.clone(),
                        );
                        workspace.clear_mappings()
                            .map_err(|e| println!("{}", e) ).ok();
                    },
                    EnvironmentUpdate::Refresh => {
                        utils::set_tables(
                            &t_env,
                            &mut tables_nb.clone(),
                            mapping_popover.clone(),
                            workspace.clone(),
                        );
                        workspace.update_mapping_data(
                            &t_env,
                            status_stack.clone()
                        ).map_err(|e| println!("{}", e) ).ok();
                    }
                }
                tables_nb.nb.set_sensitive(true);
                if query_toggle.get_active() {
                    table_toggle.set_active(true);
                }
                LayoutToolbar::clear_invalid_mappings(
                    workspace.plot_popover.clone(),
                    workspace.mapping_menus.clone(),
                    workspace.pl_view.clone()
                );
                Ok(())
            };
            sql_editor.connect_result_arrived(schema_tree.clone(), f);
        }

        let main_menu = MainMenu::build(
            &builder,
            &sql_editor,
            content_stack.clone(),
            query_toggle.clone(),
            plot_workspace.pl_view.clone(),
            tables_nb.clone(),
            table_env.clone(),
            sql_editor.clone()
        );
        plot_workspace.layout_window.connect_window_show(
            &main_menu.layout_window,
            plot_workspace.layout_path.clone()
        );
        Self {
            conn_popover,
            sql_editor,
            main_paned,
            query_toggle,
            table_env,
            tables_nb,
            table_toggle,
            plot_toggle,
            paned_pos,
            main_menu,
            plot_workspace,
            schema_tree
        }
    }

}

pub fn switch_paned(paned : Paned, show : bool) {
    if paned.get_position() == 0 {
        if show {
            paned.set_position(426);
        }
    } else {
        if !show {
            paned.set_position(0);
        }
    }
}

/*fn switch_orientation(
    paned : &Paned,
    switch_children : bool,
    new_pos : Option<(i32, i32)>,
    shrink1 : bool,
    shrink2 : bool,
    paned_pos : Rc<RefCell<i32>>
) {
    if paned.get_orientation() == Orientation::Horizontal {
        paned.set_orientation(Orientation::Vertical);
        if switch_children {
            let child1 = paned.get_child1().unwrap();
            let child2 = paned.get_child2().unwrap();
            paned.remove(&child1);
            paned.remove(&child2);
            paned.pack1(&child2, true, shrink1);
            paned.pack2(&child1, true, shrink2);
        }
        if let Some(pos) = new_pos {
            paned.set_position(pos.0);
            *(paned_pos.borrow_mut()) = pos.0;
        }
    } else {
        paned.set_orientation(Orientation::Horizontal);
        if switch_children {
            let child1 = paned.get_child1().unwrap();
            let child2 = paned.get_child2().unwrap();
            paned.remove(&child1);
            paned.remove(&child2);
            paned.pack1(&child2, true, shrink1);
            paned.pack2(&child1, true, shrink2);
        }
        if let Some(pos) = new_pos {
            paned.set_position(pos.1);
            *(paned_pos.borrow_mut()) = pos.1;
        }
    }
}*/

/*fn build_func_menu(builder : Builder) {
    let search_entry : Entry =
            builder.get_object("fn_reg_entry").unwrap();
        let completion : EntryCompletion =
            builder.get_object("fn_completion").unwrap();
        completion.set_text_column(0);
        completion.set_minimum_key_length(1);
}*/

fn build_ui(app: &gtk::Application) {
    let full = true;
    let path = if full {
        utils::glade_path("gtk-queries-full.glade").expect("Failed to load glade file")
    } else {
        utils::glade_path("gtk-queries.glade").expect("Failed to load glade file")
    };
    let builder = Builder::from_file(path);
    let win : Window = builder.get_object("main_window")
        .expect("Could not recover window");
    //win.set_destroy_with_parent(false);

    let queries_app = QueriesApp::new_from_builder(&builder, win.clone(), full);

    {
        //let toggle_q = queries_app.sql_editor.query_toggle.clone();
        let conn_popover = queries_app.conn_popover.clone();
        //let fn_popover = queries_app.fn_popover.clone();
        let tables_nb = queries_app.tables_nb.clone();
        //let toggle_w = queries_app.ws_toggle.clone();
        let view = queries_app.sql_editor.view.clone();
        let main_paned = queries_app.main_paned.clone();
        let query_toggle = queries_app.query_toggle.clone();
        let table_toggle = queries_app.table_toggle.clone();
        let plot_toggle = queries_app.plot_toggle.clone();
        let sql_stack : Stack = queries_app.sql_editor.sql_stack.clone();
        let paned_pos = queries_app.paned_pos.clone();
        let mapping_popover = queries_app.plot_workspace.layout_toolbar.mapping_popover.clone();
        //let plot_notebook = queries_app.plot_sidebar.notebook.clone();
        win.connect_key_release_event(move |_win, ev_key| {

            // ALT pressed
            if ev_key.get_state() == gdk::ModifierType::MOD1_MASK {
                if ev_key.get_keyval() == keys::constants::q {
                    //mapping_popover.hide();
                    if conn_popover.popover.get_visible() {
                        conn_popover.popover.hide();
                    }
                    if query_toggle.get_active() == false {
                        query_toggle.set_active(true);
                    }
                    if !view.borrow().get_realized() {
                        view.borrow().realize();
                    }
                    view.borrow().grab_focus();
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == keys::constants::w {
                    if conn_popover.popover.get_visible() {
                        conn_popover.popover.hide();
                    }
                    if table_toggle.get_active() == false {
                        table_toggle.set_active(true);
                    }
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == keys::constants::c {
                    conn_popover.popover.show();
                    //mapping_popover.hide();
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == keys::constants::e {
                    if conn_popover.popover.get_visible() {
                        conn_popover.popover.hide();
                    }
                    if plot_toggle.get_active() == false {
                        plot_toggle.set_active(true);
                    }
                    return glib::signal::Inhibit(true)
                }
                /*if ev_key.get_keyval() == key::f {
                    if tables_nb.selected_cols().len() > 0 {
                        if !fn_popover.is_visible() {
                            fn_popover.show();
                        } else {
                            fn_popover.hide();
                        }
                    }
                }*/
                if ev_key.get_keyval() == keys::constants::l {

                }
                return glib::signal::Inhibit(false)
            } else {
                if ev_key.get_keyval() == keys::constants::Escape {
                    mapping_popover.hide();
                    conn_popover.popover.hide();
                    table_toggle.set_active(false);
                    plot_toggle.set_active(false);
                    tables_nb.unselect_at_table();
                    //plot_notebook.set_property_page(0);
                }
                glib::signal::Inhibit(false)
            }
        });
    }

    win.set_application(Some(app));

    win.show_all();
}

// TODO change back to table when user goes to connected status message at the plot screen.

// cp queries.desktop /usr/share/applications
// cp assets/icons/queries.svg /usr/share/icons/hicolor/128x128/apps
// cp target/debug/queries /usr/bin/
fn main() -> Result<(), String> {
    gtk::init().map_err(|e| format!("{}", e) )?;

    // Required for GtkSourceView initialization from glade
    let _ = View::new();

    let app = gtk::Application::new(
        Some("com.github.limads.queries"),
        Default::default())
    .expect("Could not initialize Gtk");

    app.connect_activate(|app| {
        build_ui(app);
    });

    app.run(&args().collect::<Vec<_>>());
    Ok(())
}

// Change GtkSourceView on file set
/*{
    let view = view.clone();
    // let new_file = new_file.clone();
    // let header = header.clone();
    // let unsaved_dialog = unsaved_dialog.clone();
    let unsaved_changes = unsaved_changes.clone();
    file_btn.connect_file_set(move |btn| {
        let buffer = view.get_buffer();
        let new_file = new_file.try_borrow_mut();
        let unsaved = unsaved_changes.try_borrow_mut();
        match (buffer, new_file, unsaved) {
            (Some(mut buf), Ok(mut new_f), Ok(mut unsaved)) => {
                let from = buf.get_start_iter();
                let to = buf.get_end_iter();
                let empty_buf =  from == to;
                if (*new_f && !empty_buf) || *unsaved {
                    match unsaved_dialog.run() {
                        ResponseType::Other(0) => {
                            unsaved_dialog.hide();
                        },
                        ResponseType::Other(1) => {
                            buf.set_text("");
                            let ok = QueriesApp::load_file_to_buffer(
                                btn.clone(),
                                buf,
                                header.clone(),
                                *new_f,
                                *unsaved
                            ).is_ok();
                            if ok {
                                *new_f = false;
                                *unsaved = false;
                            }
                            unsaved_dialog.hide();
                        },
                        _ => { }
                    }
                }
            },
            _ => { println!("Unavailable reference"); }
        }
    });*/

// SourceView key release
/*{
let exec_btn = queries_app.exec_btn.clone();
let queries_app = queries_app.clone();
queries_app.clone().view.connect_key_release_event(move |view, ev| {
    queries_app.check_active_selection();
    if let Some(buf) = queries_app.view.get_buffer() {
        let from = &buf.get_start_iter();
        let to = &buf.get_end_iter();
        let old = queries_app.old_source_content.borrow();
        let unsaved = queries_app.unsaved_changes.borrow();
        if let Some(txt) = buf.get_text(from, to,true) {
            if *unsaved && txt != *old {
                let mut subtitle : String =
                    queries_app.header.get_subtitle()
                    .and_then(|s| Some(s.to_string()) )
                    .unwrap_or("".into());
                subtitle += "*";
                queries_app.header.set_subtitle(
                    Some(&subtitle[..]));
            }
        }
    }
glib::signal::Inhibit(false)
});
}*/

// Button release on GtkSourceView
/*
{
    let queries_app = queries_app.clone();
    queries_app.clone()
    .view.connect_button_release_event(move |view, ev| {
        queries_app.check_active_selection();
        glib::signal::Inhibit(false)
    });
}
*/

// Key press on GtkSourceView
/*{
    let queries_app = queries_app.clone();
    queries_app.clone()
    .view.connect_key_press_event(move |view, ev_key| {
        // check gdkkeysyms.h
        println!("{:?}", ev_key.get_keyval());

        if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK {
            if ev_key.get_keyval() == 115 {
                println!("must save now");
            }
        }

        match ev_key.get_keyval() {
            // s, i.e. CTRL+s because plain s is
            // captured and inhibited
            //if
            0x073 => {
                // queries_app.try_save_file();
            },
            // space
            0x020 => {
            }
            _ => { }
        };
        glib::signal::Inhibit(false)
    });
}*/


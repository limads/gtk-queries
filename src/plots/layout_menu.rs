use gtk::*;
use gtk::prelude::*;
// use gio::prelude::*;
use super::mapping_menu::*;
//use crate::PlotSidebar;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::{source::EnvironmentSource, environment::TableEnvironment};
use gtkplotview::plot_view::{PlotView, UpdateContent};
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use gio::FileExt;
use super::design_menu::*;
use super::scale_menu::*;
use std::collections::HashMap;
use crate::utils;
use crate::status_stack::StatusStack;
use crate::table_notebook::TableNotebook;

/// PlotsSidebar holds the information of the used mappings
#[derive(Clone)]
pub struct PlotSidebar {
    pub design_menu : DesignMenu,
    pub scale_menus : (ScaleMenu, ScaleMenu),
    pub mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
    pub notebook : Notebook,
    pub layout_stack : Stack,
    pub pl_view : Rc<RefCell<PlotView>>,
    mapping_btns : HashMap<String, ToolButton>,
    add_mapping_btn : ToolButton,
    clear_layout_btn : ToolButton,
    remove_mapping_btn : ToolButton,
    new_layout_btn : Button,
    load_layout_btn : Button,
    glade_def : Rc<String>,
    xml_load_dialog : FileChooserDialog
}

impl PlotSidebar {

    pub fn set_active(&self, state : bool) {
        // self.new_layout_btn.set_sensitive(state);
        // self.load_layout_btn.set_sensitive(state);
        if let Err(e) = self.set_add_mapping_sensitive(0) {
            println!("{}", e);
        }
        if state == false {
            self.xml_load_dialog.unselect_all();
        }
        self.layout_stack.set_visible_child_name("empty");
    }

    pub fn set_add_mapping_sensitive(&self, ncols : usize) -> Result<(), &'static str> {
        let visible = self.layout_stack.get_visible_child_name()
            .ok_or("Unable to determine layout stack status" )?;
        if &visible[..] == "layout" {
            if ncols >= 2 {
                self.add_mapping_btn.set_sensitive(true);
            } else {
                self.add_mapping_btn.set_sensitive(false);
            }
            let sensitive : Vec<&str> = match ncols {
                2 => vec!["line", "scatter", "bar"],
                3 => vec!["area", "text", "surface"],
                _ => vec![]
            };
            for (mapping, btn) in self.mapping_btns.iter() {
                if sensitive.iter().find(|n| *n == mapping).is_some() {
                    btn.set_sensitive(true);
                } else {
                    btn.set_sensitive(false);
                }
            }
        }
        Ok(())
    }

    pub fn layout_loaded(&self) -> bool {
        let sel_name = self.layout_stack.get_visible_child_name()
            .map(|n| n.to_string()).unwrap_or(String::from("empty"));
        match &sel_name[..] {
            "layout" => true,
            _ => false
        }
    }

    fn build_layout_new_btn(
        builder : Builder,
        layout_stack : Stack,
        status_stack : StatusStack,
        clear_layout_btn :ToolButton,
        plot_toggle : ToggleButton
    ) -> Button {
        let new_layout_btn : Button = builder.get_object("layout_new_btn").unwrap();
        let layout_stack = layout_stack.clone();
        {
            let layout_stack = layout_stack.clone();
            let status_stack = status_stack.clone();
            let clear_layout_btn = clear_layout_btn.clone();
            new_layout_btn.connect_clicked(move |btn| {
                layout_stack.set_visible_child_name("layout");
                status_stack.try_show_alt();
                clear_layout_btn.set_sensitive(true);
                plot_toggle.set_active(true);
            });
        }
        new_layout_btn
    }

    pub fn build_glade_def() -> Rc<String> {
        let mut def_content = String::new();
        let fpath = utils::glade_path("gtk-queries.glade").unwrap();
        if let Ok(mut f) = File::open(fpath) {
            if let Err(e) = f.read_to_string(&mut def_content) {
                panic!("{}", e);
            }
        } else {
            panic!("Error opening glade definition");
        }
        Rc::new(def_content)
    }

    pub fn new(
        builder : Builder,
        pl_view : Rc<RefCell<PlotView>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        plot_toggle : ToggleButton
    ) -> Self {
        let mapping_menus = Rc::new(RefCell::new(Vec::new()));
        let design_menu = build_design_menu(&builder, pl_view.clone());
        let plot_notebook : Notebook =
            builder.get_object("plot_notebook").unwrap();
        let scale_menus = build_scale_menus(&builder, pl_view.clone());
        let layout_stack : Stack = builder.get_object("layout_stack").unwrap();
        let glade_def = Self::build_glade_def();
        let (add_mapping_btn, clear_layout_btn, remove_mapping_btn) = Self::build_layout_toolbar(
            builder.clone(),
            status_stack.clone(),
            layout_stack.clone(),
            pl_view.clone(),
            mapping_menus.clone(),
            plot_notebook.clone()
        );
        let mapping_btns = Self::build_add_mapping_popover(
            builder.clone(),
            add_mapping_btn.clone(),
            remove_mapping_btn.clone(),
            table_env.clone(),
            pl_view.clone(),
            tbl_nb.clone(),
            glade_def.clone(),
            mapping_menus.clone(),
            plot_notebook.clone(),
            plot_toggle.clone()
        );
        let new_layout_btn = Self::build_layout_new_btn(
            builder.clone(),
            layout_stack.clone(),
            status_stack.clone(),
            clear_layout_btn.clone(),
            plot_toggle.clone()
        );
        let (load_layout_btn, xml_load_dialog) = Self::build_layout_load_button(
            glade_def.clone(),
            builder.clone(),
            pl_view.clone(),
            table_env.clone(),
            //sidebar.clone(),
            tbl_nb.clone(),
            status_stack.clone(),
            clear_layout_btn.clone(),
            plot_notebook.clone(),
            mapping_menus.clone(),
            design_menu.clone(),
            (scale_menus.0.clone(), scale_menus.1.clone()),
            plot_toggle
        );
        {
            let remove_mapping_btn = remove_mapping_btn.clone();
            plot_notebook.connect_switch_page(move |_nb, _wid, page| {
                //let page = plot_notebook.get_property_page();
                //println!("{}", page);
                if page > 2 {
                    remove_mapping_btn.set_sensitive(true);
                } else {
                    remove_mapping_btn.set_sensitive(false);
                }
            });
        }
        Self {
            design_menu,
            scale_menus,
            mapping_menus,
            notebook : plot_notebook.clone(),
            layout_stack : layout_stack.clone(),
            pl_view : pl_view.clone(),
            mapping_btns,
            add_mapping_btn,
            clear_layout_btn,
            remove_mapping_btn,
            glade_def,
            new_layout_btn,
            load_layout_btn,
            xml_load_dialog
        }
    }

    fn build_layout_toolbar(
        builder : Builder,
        status_stack : StatusStack,
        layout_stack : Stack,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_notebook : Notebook
    ) -> (ToolButton, ToolButton, ToolButton) {
        let layout_toolbar : Toolbar = builder.get_object("layout_toolbar").unwrap();
        let img_add = Image::new_from_icon_name(Some("list-add-symbolic"), IconSize::SmallToolbar);
        let img_remove = Image::new_from_icon_name(Some("list-remove-symbolic"), IconSize::SmallToolbar);
        let img_clear = Image::new_from_icon_name(Some("edit-clear-all-symbolic"), IconSize::SmallToolbar);
        let clear_layout_btn : ToolButton = ToolButton::new(Some(&img_clear), None);
        let add_mapping_btn : ToolButton = ToolButton::new(Some(&img_add), None);
        let remove_mapping_btn : ToolButton = ToolButton::new(Some(&img_remove), None);
        // TODO verify if there isn't already at least two columns selected. If there is, do not set
        // add sensititve to false.
        remove_mapping_btn.set_sensitive(false);
        add_mapping_btn.set_sensitive(false);
        clear_layout_btn.set_sensitive(false);
        layout_toolbar.insert(&clear_layout_btn, 0);
        layout_toolbar.insert(&add_mapping_btn, 1);
        layout_toolbar.insert(&remove_mapping_btn, 2);
        layout_toolbar.show_all();
        {
            let layout_stack = layout_stack.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = mapping_menus.clone();
            let notebook = plot_notebook.clone();
            clear_layout_btn.connect_clicked(move |btn| {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout.xml")));
                    layout_stack.set_visible_child_name("empty");
                    status_stack.show_curr_status();
                    if let Ok(mut mappings) = mapping_menus.try_borrow_mut() {
                        mappings.clear();
                    } else {
                        println!("Error retrieving mapping menus");
                    }
                    let children = notebook.get_children();
                    for i in 3..children.len() {
                        if let Some(c) = children.get(i) {
                            notebook.remove(c);
                        } else {
                            println!("Unable to clear notebook");
                        }
                    }
                }
                btn.set_sensitive(false);

            });
        }

        {
            let mapping_menus = mapping_menus.clone();
            let plot_view = plot_view.clone();
            let plot_notebook = plot_notebook.clone();
            remove_mapping_btn.connect_clicked(move |_| {
                    Self::remove_selected_mapping_page(
                        &plot_notebook,
                        mapping_menus.clone(),
                        plot_view.clone()
                    );
                plot_notebook.show_all();
            });
        }

        (add_mapping_btn, clear_layout_btn, remove_mapping_btn)
    }

    fn build_add_mapping_popover(
        builder : Builder,
        add_mapping_btn : ToolButton,
        remove_mapping_btn : ToolButton,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        plot_view : Rc<RefCell<PlotView>>,
        tbl_nb : TableNotebook,
        glade_def : Rc<String>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_notebook : Notebook,
        plot_toggle : ToggleButton
    ) -> HashMap<String, ToolButton> {
        let add_mapping_popover : Popover = builder.get_object("add_mapping_popover").unwrap();
        add_mapping_popover.set_relative_to(Some(&add_mapping_btn));
        let upper_mapping_toolbar : Toolbar = builder.get_object("upper_mapping_toolbar").unwrap();
        let lower_mapping_toolbar : Toolbar = builder.get_object("lower_mapping_toolbar").unwrap();
        let toolbars = [upper_mapping_toolbar, lower_mapping_toolbar];
        let mapping_names = vec![
            String::from("line"),
            String::from("scatter"),
            String::from("bar"),
            String::from("text"),
            String::from("area"),
            String::from("surface")
        ];
        let mut mapping_btns = HashMap::new();
        let iter_names = mapping_names.iter();
        for (i, mapping) in iter_names.enumerate() {
            //let mut m_name = String::from(&mapping[0..1].to_uppercase());
            //m_name += &mapping[1..];
            let img = Image::new_from_file(&(String::from("assets/icons/") +  mapping + ".svg"));
            let btn : ToolButton = ToolButton::new(Some(&img), None);
            mapping_btns.insert(mapping.to_string(), btn.clone());
            toolbars[i / 3].insert(&btn, (i % 3) as i32);
            let m = mapping.clone();
            let add_mapping_popover = add_mapping_popover.clone();
            //let builder = builder.clone();
            let tbl_env = tbl_env.clone();
            let plot_view = plot_view.clone();
            let remove_mapping_btn = remove_mapping_btn.clone();
            let tbl_nb = tbl_nb.clone();
            let glade_def = glade_def.clone();
            let mapping_menus = mapping_menus.clone();
            let plot_notebook = plot_notebook.clone();
            let plot_toggle = plot_toggle.clone();
            btn.connect_clicked(move |_btn| {
                Self::add_mapping_from_type(
                    glade_def.clone(),
                    &m[..],
                    tbl_env.clone(),
                    tbl_nb.clone(),
                    plot_view.clone(),
                    mapping_menus.clone(),
                    //builder.clone(),
                    plot_notebook.clone()
                );
                add_mapping_popover.hide();
                remove_mapping_btn.set_sensitive(true);
                plot_toggle.set_active(true);
            });
        }
        toolbars.iter().for_each(|t| t.show_all() );
        add_mapping_btn.connect_clicked(move|_btn| {
            add_mapping_popover.show();
        });
        mapping_btns
    }

    fn remove_selected_mapping_page(
        plot_notebook : &Notebook,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        let page = plot_notebook.get_property_page() as usize;
        let mapping_ix = page - 3;
        let children = plot_notebook.get_children();
        if let Some(c) = children.get(page) {
            if let Ok(mut menus) = mapping_menus.try_borrow_mut() {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    menus.remove(mapping_ix);
                    plot_notebook.remove(c);
                    let name = (mapping_ix).to_string();
                    pl_view.update(&mut UpdateContent::RemoveMapping(name));
                    for m in menus.iter_mut().skip(mapping_ix) {
                        if let Some(Ok(old_ix)) = m.get_mapping_name().map(|n| n.parse::<usize>()) {
                            println!("Old index: {} New index: {}", old_ix, old_ix - 1 );
                            m.set_mapping_name((old_ix - 1).to_string());
                        } else {
                            println!("Unable to parse mapping menu name to usize");
                        }
                    }
                } else {
                    println!("Could not get mutable reference to PlotView")
                }
            } else {
                println!("Unable to retrieve mutable reference to mapping_menus when removing page");
            }
        } else {
            println!("Invalid child position");
        }
    }

    /// Add mapping from a type string description, attributing to its
    /// name the number of mappings currently used.
    pub fn add_mapping_from_type(
        glade_def : Rc<String>,
        mapping_type : &str,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        plot_view : Rc<RefCell<PlotView>>,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        // builder_clone : Builder,
        plot_notebook : Notebook
    ) {
        let name = if let Ok(menus) = mapping_menus.try_borrow() {
            format!("{}", menus.len())
        } else {
            return;
        };
        let menu = Self::create_new_mapping_menu(
            glade_def.clone(),
            Rc::new(RefCell::new(name)),
            mapping_type.to_string(),
            data_source.clone(),
            plot_view.clone(),
            None,
            //mapping_menus.clone()
        );
        match menu {
            Ok(m) => {
                Self::append_mapping_menu(
                    m,
                    mapping_menus.clone(),
                    plot_notebook.clone(),
                    plot_view.clone(),
                    data_source.clone(),
                    tbl_nb.clone(),
                    None
                );
                println!("Mapping appended");
            },
            Err(e) => { println!("{}", e); return; }
        }
    }

    fn build_layout_load_button(
        glade_def : Rc<String>,
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        //sidebar : PlotSidebar,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        layout_clear_btn : ToolButton,
        plot_notebook : Notebook,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_toggle : ToggleButton
    ) -> (Button, FileChooserDialog) {
        let xml_load_dialog : FileChooserDialog =
            builder.get_object("xml_load_dialog").unwrap();
        let load_btn : Button=
            builder.get_object("load_layout_btn").unwrap();
        {
            let load_btn = load_btn.clone();
            let xml_load_dialog = xml_load_dialog.clone();
            load_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
            });
        }

        {
            xml_load_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            //let path = f.get_path().unwrap_or(PathBuf::new());
                            //println!("{:?}", path);
                            //if let Some(path) = f {
                            let new_mapping_info = match plot_view.try_borrow_mut() {
                                Ok(mut pl) => {
                                    match pl.plot_area.load_layout(path.to_str().unwrap_or("").into()) {
                                        Ok(_) => Some(pl.plot_area.mapping_info()),
                                        Err(e) => { println!("{}", e); None }
                                    }
                                },
                                Err(_) => { println!("Could not get reference to Plot widget"); None }
                            };
                            if let Some(new_info) = new_mapping_info {
                                Self::clear_mappings(
                                    mapping_menus.clone(),
                                    plot_notebook.clone()
                                ).expect("Error clearing mappings");
                                Self::update_layout_widgets(
                                    design_menu.clone(),
                                    scale_menus.clone(),
                                    plot_view.clone()
                                );
                                layout_clear_btn.set_sensitive(true);
                                for m_info in new_info.iter() {
                                    let menu = Self::create_new_mapping_menu(
                                        glade_def.clone(),
                                        //builder.clone(),
                                        Rc::new(RefCell::new(m_info.0.clone())),
                                        m_info.1.clone(),
                                        data_source.clone(),
                                        plot_view.clone(),
                                        Some(m_info.2.clone()),
                                        //sidebar.clone()
                                    );
                                    match menu {
                                        Ok(m) => {
                                            Self::append_mapping_menu(
                                                m,
                                                mapping_menus.clone(),
                                                plot_notebook.clone(),
                                                plot_view.clone(),
                                                data_source.clone(),
                                                tbl_nb.clone(),
                                                None
                                            );
                                        },
                                        Err(e) => { println!("{}", e); return; }
                                    }
                                }
                                plot_notebook.show_all();
                                status_stack.try_show_alt();
                                plot_toggle.set_active(true);
                                // sidebar.layout_stack.set_visible_child_name("layout");
                                // println!("{:?}", mappings);
                            } else {
                                println!("No info to update");
                            }
                        } else {
                            println!("Could not get filename from dialog");
                        }
                    },
                    _ => { }
                }
            });
        }
        (load_btn, xml_load_dialog)
    }

    /// The creation of a mapping menu is based on an id naming convention
    /// of passing a prefix identifying the mappping (line, scatter, box, etc)
    /// followed by an element identifier. This convention applies to the enclosing box
    /// (line_box, scatter_box ...) and its constituint widgets (scatter_color_button,
    /// line_color_button) and so on. The builder for each mapping menu must be unique
    /// to avoid aliasing.
    /// Make this mapping_menu::create(.)
    fn create_new_mapping_menu(
        glade_def : Rc<String>,
        mapping_name : Rc<RefCell<String>>,
        mapping_type : String,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        pl_view : Rc<RefCell<PlotView>>,
        properties : Option<HashMap<String, String>>,
        //sidebar : PlotSidebar
    ) -> Result<MappingMenu, &'static str> {
        //println!("{}", *glade_def);
        //let builder = Builder::new_from_string(&glade_def[..]);
        let builder = Builder::new_from_file(utils::glade_path("gtk-queries.glade").unwrap());
        //println!("{:?}", builder);
        let valid_mappings = ["line", "scatter", "bar", "area", "text", "surface"];
        if !valid_mappings.iter().any(|s| &mapping_type[..] == *s) {
            return Err("Invalid mapping type. Must be line|scatter|bar|area|text|surface");
        }
        let box_name = mapping_type.clone() + "_box";
        let mapping_box : Box = builder.get_object(&box_name).unwrap();
        let design_widgets = HashMap::new();
        let ixs = Rc::new(RefCell::new(Vec::new()));
        let mut m = MappingMenu {
            mapping_name,
            mapping_type,
            mapping_box,
            design_widgets,
            ixs
        };
        m.build_mapping_design_widgets(
            &builder,
            pl_view.clone()
        );

        if let Some(prop) = properties {
            if let Err(e) = m.update_widget_values(prop) {
                println!("{}", e);
            }
        }
        Ok(m)
    }

    fn create_tab_image(m_type : String) -> Image {
        let tab_img_path = String::from("assets/icons/") + &m_type + ".svg";
        Image::new_from_file(&tab_img_path[..])
    }

    fn append_mapping_menu(
        mut m : MappingMenu,
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        notebook : Notebook,
        plot_view : Rc<RefCell<PlotView>>,
        tbl_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        pos : Option<usize>
    ) {
        match (plot_view.try_borrow_mut(), tbl_env.try_borrow(), mappings.try_borrow_mut()) {
            (Ok(mut pl), Ok(t_env), Ok(mut mappings)) => {
                //m.update_available_cols(source.col_names(), &pl);
                match pos {
                    Some(p) => mappings.insert(p, m.clone()),
                    None => mappings.push(m.clone())
                }
                let tab_img = Self::create_tab_image(m.mapping_type.clone());
                notebook.add(&m.get_parent());
                notebook.set_tab_label(&m.get_parent(), Some(&tab_img));
                let npages = notebook.get_children().len() as i32;
                notebook.set_property_page(npages-1);
                notebook.show_all();
                if let Ok(name) = m.mapping_name.try_borrow() {
                    pl.update(&mut UpdateContent::NewMapping(
                        name.clone(),
                        m.mapping_type.to_string())
                    );
                    let selected = tbl_nb.full_selected_cols();
                    let cols = t_env.get_columns(&selected[..]);
                    println!("{:?}", cols);
                    if let Err(e) = m.update_data(selected, cols, &mut pl) {
                        println!("{}", e);
                        return;
                    }
                } else {
                    println!("Unable to retrive reference to mapping name");
                }
            },
            (_,_,Err(e)) => { println!("{}", e); },
            _ => {
                println!("Unable to retrieve mutable reference to plot view|data source");
            }
        }
    }

    pub fn clear(&self) {
        if let Err(e) = Self::clear_mappings(self.mapping_menus.clone(), self.notebook.clone()) {
            println!("{}", e);
        }
        if let Ok(mut pl_view) = self.pl_view.try_borrow_mut() {
            pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout.xml")));
        } else {
            println!("Failed to borrow mutable reference to plotview.");
        }
    }

    fn clear_mappings(
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        plot_notebook : Notebook
    ) -> Result<(), &'static str> {
        if let Ok(mut mappings) = mappings.try_borrow_mut() {
            for m in mappings.iter() {
                plot_notebook.remove(&m.get_parent());
            }
            mappings.clear();
            Ok(())
        } else {
            Err("Could not fetch mutable reference to mapping menus before clearing them")
        }
    }

    fn update_layout_widgets(
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_view : Rc<RefCell<PlotView>>
    ) {
        match plot_view.try_borrow_mut() {
            Ok(pl) => {
                design_menu.update(pl.plot_area.design_info());
                scale_menus.0.update(pl.plot_area.scale_info("x"));
                scale_menus.1.update(pl.plot_area.scale_info("y"));
            },
            _ => {
                panic!("Could not fetch plotview reference to update layout");
            }
        }
    }


}

/*
/*// LayoutMenu encapsulate the logic of the buttons at the bottom-left
// that allows changing the plot layout and mappings.
#[derive(Clone)]
pub struct LayoutMenu {

    new_layout_btn : Button,
    // add_mapping_btn : ToolButton,
    // manage_btn : Button,
    // remove_mapping_btn : ToolButton,
    layout_stack : Stack,
    glade_def : Rc<String>,

    //manage_mapping_popover : Popover
}*/

/*fn load_text_content(path : PathBuf)
-> Option<String> {
    if let Ok(mut f) = File::open(path) {
        let mut content = String::new();
        let has_read = f.read_to_string(&mut content);
        if has_read.is_ok() {
            return Some(content);
        } else {
            None
        }
    } else {
        None
    }
}*/

//impl LayoutMenu {

    /*fn build_layout_load_button(
        glade_def : Rc<String>,
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        sidebar : PlotSidebar,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        layout_clear_btn : ToolButton
    ) -> Button {
        let xml_load_dialog : FileChooserDialog =
            builder.get_object("xml_load_dialog").unwrap();
        let load_btn : Button=
            builder.get_object("load_layout_btn").unwrap();
        {
            let load_btn = load_btn.clone();
            let xml_load_dialog = xml_load_dialog.clone();
            load_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
            });
        }

        {
            xml_load_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            //let path = f.get_path().unwrap_or(PathBuf::new());
                            //println!("{:?}", path);
                            //if let Some(path) = f {
                            let new_mapping_info = match plot_view.try_borrow_mut() {
                                Ok(mut pl) => {
                                    match pl.plot_area.load_layout(path.to_str().unwrap_or("").into()) {
                                        Ok(_) => Some(pl.plot_area.mapping_info()),
                                        Err(e) => { println!("{}", e); None }
                                    }
                                },
                                Err(_) => { println!("Could not get reference to Plot widget"); None }
                            };
                            if let Some(new_info) = new_mapping_info {
                                Self::clear_mappings(
                                    sidebar.mapping_menus.clone(),
                                    sidebar.notebook.clone()
                                ).expect("Error clearing mappings");
                                Self::update_layout_widgets(
                                    sidebar.clone(),
                                    plot_view.clone()
                                );
                                layout_clear_btn.set_sensitive(true);
                                for m_info in new_info.iter() {
                                    let menu = Self::create_new_mapping_menu(
                                        glade_def.clone(),
                                        //builder.clone(),
                                        Rc::new(RefCell::new(m_info.0.clone())),
                                        m_info.1.clone(),
                                        data_source.clone(),
                                        plot_view.clone(),
                                        Some(m_info.2.clone()),
                                        sidebar.clone()
                                    );
                                    match menu {
                                        Ok(m) => {
                                            Self::append_mapping_menu(
                                                m,
                                                sidebar.mapping_menus.clone(),
                                                sidebar.notebook.clone(),
                                                plot_view.clone(),
                                                data_source.clone(),
                                                tbl_nb.clone(),
                                                None
                                            );
                                        },
                                        Err(e) => { println!("{}", e); return; }
                                    }
                                }
                                sidebar.notebook.show_all();
                                status_stack.try_show_alt();
                                // sidebar.layout_stack.set_visible_child_name("layout");
                                // println!("{:?}", mappings);
                            } else {
                                println!("No info to update");
                            }
                        } else {
                            println!("Could not get filename from dialog");
                        }
                    },
                    _ => { }
                }
            });
        }
        load_btn
    }*/

    /*fn selected_mapping_radio(scatter_radio : &RadioButton) -> Option<String> {
        for radio in scatter_radio.get_group() {
            if radio.get_active() {
                if let Some(name) = WidgetExt::get_widget_name(&radio) {
                    return Some(name.as_str().to_string());
                }
            }
        }
        None
    }

    fn set_mapping_radio(scatter_radio : &RadioButton, curr_type : String) {
        for radio in scatter_radio.get_group() {
            if let Some(name) = WidgetExt::get_widget_name(&radio) {
                if name == &curr_type[..] {
                    radio.set_active(true);
                    return;
                }
            }
        }
        println!("Radio not found for informed type");
    }*/

    /*pub fn new_from_builder(
        builder : &Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        sidebar : PlotSidebar
    ) -> Self {





        Self {
            load_layout_btn,
            add_mapping_btn,
            new_layout_btn,
            remove_mapping_btn,
            layout_stack,
            glade_def,
            mapping_btns
            //manage_mapping_popover
        }
    }

}*/

*/

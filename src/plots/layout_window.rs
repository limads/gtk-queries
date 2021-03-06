use gtk::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::tables::environment::TableEnvironment;
use crate::plots::plotview::GroupSplit;
use crate::plots::plotview::plot_view::{PlotView, UpdateContent};
use std::fs::File;
use super::design_menu::*;
use super::scale_menu::*;
use super::layout_toolbar::*;
use super::mapping_menu::*;
use std::collections::HashMap;
use crate::table_notebook::TableNotebook;
use crate::status_stack::*;
use super::plot_workspace::PlotWorkspace;
use std::io::Write;
use std::path::Path;
use crate::utils::RecentList;
use gdk::RGBA;
use crate::utils;
use gdk_pixbuf::Pixbuf;
use super::scale_menu;

const ALL_LAYOUTS : [GroupSplit; 8] = [
    GroupSplit::Unique,
    GroupSplit::Vertical,
    GroupSplit::Horizontal,
    GroupSplit::Four,
    GroupSplit::ThreeLeft,
    GroupSplit::ThreeTop,
    GroupSplit::ThreeRight,
    GroupSplit::ThreeBottom
];

const ALL_PATHS : [&'static str; 8] = [
    "unique",
    "vert",
    "horiz",
    "four",
    "three-left",
    "three-top",
    "three-right",
    "three-bottom"
];

#[derive(Clone)]
pub struct LayoutWindow {
    pub open_btn : Button,
    pub save_btn : Button,
    // pub query_btn : Button,
    pub clear_btn : Button,
    pub xml_save_dialog : FileChooserDialog,
    pub xml_load_dialog : FileChooserDialog,
    toggles : HashMap<GroupSplit, ToggleToolButton>,
    file_combo : ComboBoxText,

    // Holds (File, Recent paths, file_updated)
    pub recent : RecentList,
    pub horiz_ar_scale : Scale,
    pub vert_ar_scale : Scale,
    dim_combo : ComboBoxText,
    layout_list_box : ListBox,
    pub mapping_tree : MappingTree,
    pub win : Window,
    pub layout_stack : Stack
}

/*#[derive(Debug, Clone)]
pub struct MappingRow {
    row : ListBoxRow
}*/

#[derive(Debug, Clone)]
pub struct MappingTree {
    icons : HashMap<String, Pixbuf>,
    pub tree_view : TreeView,
    model : TreeStore,
    stack : Stack,
    // rows : Vec<ListBoxRow>,
    scale_menus : (ScaleMenu, ScaleMenu),
    line_menu : LineMenu,
    scatter_menu : ScatterMenu,
    bar_menu : BarMenu,
    text_menu : TextMenu,
    area_menu : AreaMenu,
    surface_menu : SurfaceMenu,
    
    // Name of the currently-set mapping
    mapping_name : Rc<RefCell<String>>
}

impl MappingTree {

    fn build(builder : &Builder, plot_view : Rc<RefCell<PlotView>>) -> Self {
        let icons = Self::load_mapping_icons();
        let tree_view : TreeView = builder.get_object("mapping_tree_view").unwrap();
        let model = utils::configure_tree_view(&tree_view);
        let stack : Stack = builder.get_object("mapping_window_stack").unwrap();
        let scale_menus = scale_menu::build_scale_menus(&builder, plot_view.clone());
        let mapping_name = Rc::new(RefCell::new(String::new()));
        let line_menu = LineMenu::build(&builder);
        line_menu.hook(plot_view.clone(), mapping_name.clone());
        let scatter_menu = ScatterMenu::build(&builder);
        scatter_menu.hook(plot_view.clone(), mapping_name.clone());
        let bar_menu = BarMenu::build(&builder);
        bar_menu.hook(plot_view.clone(), mapping_name.clone());
        let text_menu = TextMenu::build(&builder);
        text_menu.hook(plot_view.clone(), mapping_name.clone());
        let area_menu = AreaMenu::build(&builder);
        area_menu.hook(plot_view.clone(), mapping_name.clone());
        let surface_menu = SurfaceMenu::build(&builder);
        surface_menu.hook(plot_view.clone(), mapping_name.clone());
        
        {
            let scale_menus = scale_menus.clone();
            let mapping_name = mapping_name.clone();
            let line_menu = line_menu.clone();
            let scatter_menu = scatter_menu.clone();
            let text_menu = text_menu.clone();
            let bar_menu = bar_menu.clone();
            let area_menu = area_menu.clone();
            let surface_menu = surface_menu.clone();
            let stack = stack.clone();
            tree_view.connect_cursor_changed(move |view| {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    if let (Some(path), _) = view.get_cursor() {
                        println!("Depth: {}", path.get_depth());
                        println!("Indices: {:?}", path.get_indices());
                        pl_view.set_active_area(path.get_indices()[0] as usize);
                        // Depth=1 is a subplot selection; Depth == 2 is a mapping selection.
                        if path.get_depth() == 1 {
                            let info_x = pl_view.current_scale_info("x");
                            let info_y = pl_view.current_scale_info("y");
                            scale_menus.0.update(info_x);
                            scale_menus.1.update(info_y);
                            stack.set_visible_child_name("scale");
                        } else {
                            println!("Indices: {:?}", path.get_indices());
                            let mapping_ix = path.get_indices()[1] as usize;
                            pl_view.set_active_area(path.get_indices()[0] as usize);
                            let info = pl_view.mapping_info();
                            let (name, ty, props) = info[mapping_ix].clone();
                            // let mut names = info.iter().map(|(name, _, _)| name );
                            
                            // Associating the current selected name here makes changes
                            // to the mapping widgets propagate to the right mapping name
                            // (established at mapping::hook(.)
                            *(mapping_name.borrow_mut()) = name;
                            match &ty[..] {
                                "line" => line_menu.update(props),
                                "scatter" => scatter_menu.update(props),
                                "text" => text_menu.update(props),
                                "area" => area_menu.update(props),
                                "bar" => bar_menu.update(props),
                                "surface" => surface_menu.update(props),
                                _ => println!("Invalid mapping")
                            }
                            stack.set_visible_child_name(&ty);
                        }
                    }            
                } else {
                    println!("Unable to borrow plot view");
                }
            });
        }
        
        Self { 
            icons, 
            tree_view, 
            model,
            stack, 
            scale_menus,
            line_menu,
            scatter_menu,
            bar_menu,
            text_menu,
            area_menu,
            surface_menu,
            mapping_name
        }
    }
    
    pub fn append_mapping(&self, subplot : usize, ty : &str, props : &HashMap<String, String>) {
        let iter = self.model.iter_nth_child(None, subplot as i32).unwrap();
        let mapping_pos = self.model.append(Some(&iter));
        self.insert_mapping_row(ty, &props, &mapping_pos);
    }
    
    pub fn remove_mapping(&self, subplot : usize, mapping_ix : usize) {
        let top = self.model.iter_nth_child(None, subplot as i32).unwrap();
        if let Some(iter) = self.model.iter_nth_child(Some(&top), mapping_ix as i32) {
            self.model.remove(&iter);
            self.tree_view.show_all();
        } else {
            println!("Tried to remove plot {} of area {} but it was not available", mapping_ix, subplot);
        }
    }
    
    fn load_mapping_icons() -> HashMap<String, Pixbuf> {
        let mut mapping_icons = HashMap::new();
        let mappings = ["line", "scatter", "area", "text", "surface", "bar"];
        let plots = [
            "layout/unique-1",
            "layout/horiz-1",
            "layout/horiz-2",
            "layout/vert-1",
            "layout/vert-2",
            "layout/three-left-1",
            "layout/three-left-2",
            "layout/three-left-3",
            "layout/three-top-1",
            "layout/three-top-2",
            "layout/three-top-3",
            "layout/three-right-1",
            "layout/three-right-2",
            "layout/three-right-3",
            "layout/three-bottom-1",
            "layout/three-bottom-2",
            "layout/three-bottom-3",
            "layout/four-1",
            "layout/four-2",
            "layout/four-3",
            "layout/four-4",
        ];
        for m in mappings.iter().chain(plots.iter()) {
            let pix = Pixbuf::from_file_at_scale(&format!("assets/icons/{}.svg", m), 16, 16, true).unwrap();
            mapping_icons.insert(format!("{}", m), pix);
        }
        mapping_icons
    }
    
    fn insert_mapping_row(&self, ty : &str, props : &HashMap<String,String>, mapping_pos : &TreeIter) {
        let mapping_name = match &ty[..] {
            "area" => format!("{} x {} x {}", props["x"], props["ymin"], props["ymax"]), 
            "text" => format!("{} x {} x {}", props["x"], props["y"], props["text"]),
            "surface" => format!("{} x {} x {}", props["x"], props["y"], props["z"]),
            "bar" => format!("{}", props["height"]),
            _ => format!("{} x {}", props["x"], props["y"])
        };
        self.model.set(&mapping_pos, &[0, 1], &[&self.icons[&ty[..]], &mapping_name.to_value()]);
        self.tree_view.show_all();
    }
    
    pub fn repopulate(&self, pl_view : Rc<RefCell<PlotView>>) {
        self.clear();
        if let Ok(mut pl) = pl_view.try_borrow_mut() {
            let split = pl.group_split();
            self.reset_subplots(&split);
            let mut curr_subplot = 0;
            
            // Iterate over subplots (set at reset_subplots) and add all mappings for each one.
            self.model.foreach(move |model, path, iter| {
                let is_subplot_node = path.get_indices().len() == 1;
                if is_subplot_node {
                    pl.set_active_area(curr_subplot);
                    let info = pl.mapping_info();
                    for (name, ty, props) in info {
                        let mapping_pos = self.model.append(Some(iter));
                        let mapping_name = self.insert_mapping_row(&ty, &props, &mapping_pos);
                    }
                    curr_subplot += 1;
                }
                true
            });
        } else {
            println!("Unable to mutably borrow plot view");
        }
        self.tree_view.expand_all();
        self.tree_view.show_all();
    }
    
    fn reset_subplots(&self, split : &GroupSplit) {
        self.clear();
        let subplot_names = match split {
            GroupSplit::Unique => vec![("Center", "layout/unique-1")],
            GroupSplit::Horizontal => vec![("Left", "layout/horiz-1"), ("Right", "layout/horiz-2")],
            GroupSplit::Vertical => vec![("Top", "layout/vert-1"), ("Bottom", "layout/vert-2")],
            GroupSplit::ThreeLeft => {
                vec![
                    ("Left", "layout/three-left-1"), 
                    ("Top right", "layout/three-left-2"), 
                    ("Bottom right", "layout/three-left-3")
                ]
            },
            GroupSplit::ThreeTop => {
                vec![
                    ("Top", "layout/three-top-1"), 
                    ("Bottom left", "layout/three-top-2"), 
                    ("Bottom right", "layout/three-top-3")
                ]
            },
            GroupSplit::ThreeRight => {
                vec![
                    ("Top left", "layout/three-right-1"), 
                    ("Right", "layout/three-right-2"), 
                    ("Bottom left", "layout/three-right-3")
                ]
            },
            GroupSplit::ThreeBottom => {
                vec![
                    ("Top Left", "layout/three-bottom-1"), 
                    ("Top right", "layout/three-bottom-2"), 
                    ("Bottom", "layout/three-bottom-3")
                ]
            },
            GroupSplit::Four =>{
                vec![
                    ("Top Left", "layout/four-1"), 
                    ("Top right", "layout/four-2"), 
                    ("Bottom left", "layout/four-3"),
                    ("Bottom-right", "layout/four-4")
                ]
            }
        };
        for (name, icon_name) in subplot_names.iter() {
            // let opt_parent = self.model.get_iter_first();
            let schema_pos = self.model.append(None);
            self.model.set(&schema_pos, &[0, 1], &[&self.icons[&icon_name[..]], &name.to_value()]);
        }
        self.tree_view.show_all();
    }
    
    pub fn clear(&self) {
        self.model.clear();
        self.tree_view.show_all();
    }
    
    pub fn set_selected(&self, plot_ix : usize, mapping_ix : usize) {
        println!("Indices: [{}, {}]", plot_ix, mapping_ix);
        // let selection = self.tree_view.get_selection();
        let path = TreePath::from_indicesv(&[plot_ix as i32, mapping_ix as i32]);
        // selection.select_path(&path);
        let no_col : Option<&TreeViewColumn> = None;
        self.tree_view.set_cursor(&path, no_col, false);
    }
        
}

impl LayoutWindow {

    pub fn update_recent_paths(
        file_combo : ComboBoxText,
        recent : &RecentList,
        layout_path : Rc<RefCell<Option<String>>>
    ) {
        // recent.load_recent_paths();
        let mut opt_active : Option<String> = None;
        if let Ok(opt_path) = layout_path.try_borrow() {
            file_combo.remove_all();
            for (i, path) in recent.loaded_items().iter().enumerate() {
                let id = format!("{}", i);
                file_combo.append(Some(&id[..]), &path[..]);
                if let Some(current_path) = &*opt_path {
                    if &path[..] == &current_path[..] {
                        opt_active = Some(id.clone());
                    }
                }
            }
        } else {
            println!("Could not read recent file contents");
        }
        if let Some(active_id) = opt_active {
            // file_combo.set_active_id(Some(&active_id[..]));
            file_combo.set_active_id(None);
        } else {
            file_combo.set_active_iter(None);
        }
    }

    pub fn connect_window_show(&self, /*win : &Window,*/ layout_path : Rc<RefCell<Option<String>>>) {
        let file_combo = self.file_combo.clone();
        let recent = self.recent.clone();
        self.win.connect_show(move |_| {
            Self::update_recent_paths(
                file_combo.clone(),
                &recent,
                layout_path.clone()
            );
        });
    }

    /*pub fn set_sensitive_at_index(menus : &[MappingMenu], ix : usize) {
        for m in menus.iter() {
            if m.plot_ix == ix {
                m.tab_img.set_sensitive(true);
                m.set_sensitive(true);
            } else {
                m.tab_img.set_sensitive(false);
                m.set_sensitive(false);
            }
        }
    }*/

    fn build_save_dialog(
        builder : &Builder,
        save_btn : Button,
        // layout_file_combo : ComboBoxText,
        pl_view : Rc<RefCell<PlotView>>,
        file_combo : ComboBoxText,
        recent : RecentList,
        layout_path : Rc<RefCell<Option<String>>>
    ) -> FileChooserDialog {
        let xml_save_dialog : FileChooserDialog = builder.get_object("xml_save_dialog").unwrap();
        xml_save_dialog.connect_response(move |dialog, resp| {
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        let ext = path.as_path()
                            .extension()
                            .map(|ext| ext.to_str().unwrap_or(""));
                        if let Some(ext) = ext {
                            match ext {
                                "xml" => {
                                    if let Ok(pl) = pl_view.try_borrow() {
                                        if let Ok(mut f) = File::create(&path) {
                                            let content = pl.plot_group.get_layout_as_text();
                                            if let Err(e) = f.write_all(&content.into_bytes()) {
                                                println!("{}", e);
                                                return;
                                            }
                                            pl.parent.queue_draw();
                                        } else {
                                            println!("Unable to create file");
                                            return;
                                        }
                                    } else {
                                        println!("Unable to retrieve reference to plot");
                                        return;
                                    }
                                    let path_str = path.to_str()
                                        .map(|s| s.to_string())
                                        .unwrap_or(String::new());
                                    recent.push_recent(path_str.clone());
                                    *(layout_path.borrow_mut()) = Some(path_str.clone());
                                    Self::update_recent_paths(
                                        file_combo.clone(),
                                        &recent,
                                        layout_path.clone()
                                    );
                                },
                                _ => { println!("Layout extension should be .xml"); }
                            }
                        }
                    }
                },
                _ => { }
            }
        });
        {
            let xml_save_dialog = xml_save_dialog.clone();
            save_btn.connect_clicked(move |_btn| {
                xml_save_dialog.run();
                xml_save_dialog.hide();
            });
        }
        xml_save_dialog
    }

    /*pub fn populate_list_box(layout_list_box : &ListBox, wid : &impl IsA<Widget>, name : &str) {
        let row = ListBoxRow::new();
        let bx = Box::new(Orientation::Horizontal, 0);
        let lbl = Label::new(Some("Dimensions"));
        lbl.set_margin_start(6);
        lbl.set_margin_end(6);
        lbl.set_margin_top(6);
        lbl.set_margin_bottom(6);
        lbl.set_justify(Justification::Left);
        bx.pack_start(&lbl, true, true, 0);
        // let entry = Entry::new();
        // entry.set_max_length(16);
        // bx.pack_start(&entry, false, false, 0);
        /*let img_with = Image::from_file(
            Some("assets/"),
            IconSize::SmallToolbar
        );*/
        bx.pack_start(wid, false, false, 0);
        row.add(&bx);
        row.set_selectable(false);
        row.set_activatable(false);
        // row.set_margin_top(6);
        // row.set_margin_bottom(6);
        let n = layout_list_box.get_children().len();
        layout_list_box.insert(&row, n as i32);
        layout_list_box.show_all();
        row.set_property_height_request(64);
    }*/
    
    pub fn new(
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        // mapping_stack : Stack,
        layout_path : Rc<RefCell<Option<String>>>,
        layout_group_toolbar : GroupToolbar
    ) -> LayoutWindow {
        let win : Window = builder.get_object("layout_window").unwrap();
        let mapping_tree = MappingTree::build(&builder, plot_view.clone());
        let layout_list_box : ListBox = builder.get_object("layout_list_box").unwrap();
        let dim_combo : ComboBoxText = builder.get_object("dim_combo").unwrap();
        /*dim_combo.append(Some("0"), "480 x 320");
        dim_combo.append(Some("1"), "800 x 600");
        dim_combo.append(Some("2"), "1080 x 960");
        dim_combo.set_active_id(Some("0"));
        dim_combo.set_margin_start(6);
        dim_combo.set_margin_end(6);
        dim_combo.set_margin_top(6);
        dim_combo.set_margin_bottom(6);
        Self::populate_list_box(&layout_list_box, &dim_combo, "Dimension");*/
        
        let layout_toggle_layout : ToggleButton = builder.get_object("layout_toggle_layout").unwrap();
        let layout_toggle_mapping : ToggleButton = builder.get_object("layout_toggle_mapping").unwrap();
        let layout_stack : Stack = builder.get_object("layout_window_stack").unwrap();
        
        {
            let layout_stack = layout_stack.clone();
            let layout_toggle_mapping = layout_toggle_mapping.clone(); 
            layout_toggle_layout.connect_toggled(move |btn| {
                if btn.get_active() {
                    layout_stack.set_visible_child_name("layout");
                    layout_toggle_mapping.set_active(false);
                }
            });
        }
        
        {
            let layout_stack = layout_stack.clone();
            let layout_toggle_layout = layout_toggle_layout.clone();
            layout_toggle_mapping.connect_toggled(move |btn| {
                if btn.get_active() {
                    layout_stack.set_visible_child_name("mapping");
                    layout_toggle_layout.set_active(false);
                }
            });
        }
         
        let group_toolbar : Toolbar = builder.get_object("group_toolbar").unwrap();
        group_toolbar.override_background_color(StateFlags::NORMAL, Some(&RGBA::white()));
        
        // let group_toolbar_bottom : Toolbar = builder.get_object("group_toolbar_bottom").unwrap();
        // let toolbars : [Toolbar; 2] = [group_toolbar_top.clone(), group_toolbar_bottom.clone()];

        let horiz_ar_scale : Scale = builder.get_object("horiz_ar_scale").unwrap();
        let vert_ar_scale : Scale = builder.get_object("vert_ar_scale").unwrap();

        {
            let plot_view = plot_view.clone();
            horiz_ar_scale.get_adjustment().connect_value_changed(move |adj : &Adjustment| {
                let val_horiz = adj.get_value() / 100.0;
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::AspectRatio(Some(val_horiz), None));
                } else {
                    println!("Failed acquiring reference to plot view");
                }
            });
        }

        {
            let plot_view = plot_view.clone();
            vert_ar_scale.get_adjustment().connect_value_changed(move |adj : &Adjustment| {
                let val_vert = adj.get_value() / 100.0;
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::AspectRatio(None, Some(val_vert)));
                } else {
                    println!("Failed acquiring reference to plot view");
                }
            });
        }

        let mut toggles = HashMap::new();
        let layout_iter = ALL_LAYOUTS.iter().zip(ALL_PATHS.iter());
        for (i, (layout, path)) in layout_iter.clone().enumerate() {
            let img = Image::from_file(&(String::from("assets/icons/layout-") + path + ".svg"));
            let btn : ToggleToolButton = ToggleToolButton::new();
            btn.set_icon_widget(Some(&img));
            toggles.insert(*layout, btn.clone());
            group_toolbar.insert(&btn, i as i32);
        }
        // group_toolbar_bottom.show_all();

        for (layout, path) in layout_iter {
            let toggles = toggles.clone();
            let plot_view = plot_view.clone();
            // let mapping_menus = mapping_menus.clone();
            // let mapping_stack = mapping_stack.clone();
            let horiz_ar_scale = horiz_ar_scale.clone();
            let vert_ar_scale = vert_ar_scale.clone();
            let layout_group_toolbar = layout_group_toolbar.clone();
            let mapping_tree = mapping_tree.clone();
            toggles[layout].clone().connect_toggled(move |curr_toggle| {
                if curr_toggle.get_active() {
                    toggles.iter()
                        .filter(|(k, _)| *k != layout )
                        .for_each(|(_, btn)|{ btn.set_active(false) });
                        
                    /*PlotWorkspace::clear_mapping_widgets(
                        mapping_menus.clone(),
                        mapping_stack.clone()
                    ).expect("Error clearing mappings");*/
                    mapping_tree.reset_subplots(&layout);
                    
                    // TODO load all layouts at beginning.
                    if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                        pl_view.change_active_area(0);
                        let clear_path = format!("assets/plot_layout/layout-{}.xml", path);
                        if let Err(e) = pl_view.update(&mut UpdateContent::Clear(clear_path)) {
                            println!("{}", e);
                        }
                    } else {
                        println!("Unable to get mutable reference to plotview");
                    }
                    layout_group_toolbar.set_active_default(Some(*layout));
                    match layout {
                        GroupSplit::Unique => {
                            horiz_ar_scale.set_sensitive(false);
                            vert_ar_scale.set_sensitive(false);
                        },
                        GroupSplit::Horizontal => {
                            horiz_ar_scale.set_sensitive(true);
                            vert_ar_scale.set_sensitive(false);
                        },
                        GroupSplit::Vertical => {
                            horiz_ar_scale.set_sensitive(false);
                            vert_ar_scale.set_sensitive(true);
                        },
                        _ => {
                            horiz_ar_scale.set_sensitive(true);
                            vert_ar_scale.set_sensitive(true);
                        }
                    }
                    horiz_ar_scale.set_value(50.);
                    vert_ar_scale.set_value(50.);

                } else {
                    // toggles[&GroupSplit::Unique].set_active(true);
                }
            });
        }

        toggles[&GroupSplit::Unique].set_active(true);
        group_toolbar.show_all();
        
        let file_combo : ComboBoxText = builder.get_object("layout_file_combo").unwrap();
        let recent = RecentList::new(Path::new("assets/plot_layout/recent_paths.csv"), 11).unwrap();
        Self::update_recent_paths(file_combo.clone(), &recent, layout_path.clone());
        let open_btn : Button = builder.get_object("layout_open_btn").unwrap();
        let save_btn : Button = builder.get_object("layout_save_btn").unwrap();
        let clear_btn : Button = builder.get_object("layout_clear_btn").unwrap();
        // let query_btn : Button = builder.get_object("layout_query_btn").unwrap();

        let xml_load_dialog : FileChooserDialog = builder.get_object("xml_load_dialog").unwrap();
        let xml_save_dialog = Self::build_save_dialog(
            &builder,
            save_btn.clone(),
            // file_combo.clone(),
            plot_view.clone(),
            file_combo.clone(),
            recent.clone(),
            layout_path.clone()
        );

        /*{
            let xml_load_dialog = xml_load_dialog.clone();
            let plot_view = plot_view.clone();
            open_btn.connect_clicked(move |btn| {
                xml_load_dialog.show();
                xml_load_dialog.hide();
            });
        }*/

        {
            let xml_save_dialog = xml_save_dialog.clone();
            save_btn.connect_clicked(move |btn| {
                xml_save_dialog.show();
                xml_save_dialog.hide();
            });
        }

        // let layout_width_entry : Entry = builder.get_object("layout_width_entry").unwrap();
        // let layout_height_entry : Entry = builder.get_object("layout_height_entry").unwrap();
        
        /*{
            let plot_view = plot_view.clone();
            layout_width_entry.connect_focus_out_event(move |entry, _ev| {
                let txt = entry.get_text();
                if let Ok(mut pl) = plot_view.try_borrow_mut() {
                    if let Ok(w) = txt.parse::<usize>() {
                        if let Err(e) = pl.update(&mut UpdateContent::Dimensions(Some(w), None)) {
                            println!("{}", e);
                        }
                    } else {
                        println!("Unable to borrow field as text");
                    }
                } else {
                    println!("Unable to borrow plotview");
                }
                glib::signal::Inhibit(true)
            });
        }*/
        
        /*{
            let plot_view = plot_view.clone();
            layout_height_entry.connect_focus_out_event(move |entry, _ev| {
                let txt = entry.get_text();
                if let Ok(mut pl) = plot_view.try_borrow_mut() {
                    if let Ok(h) = txt.parse::<usize>() {
                        if let Err(e) = pl.update(&mut UpdateContent::Dimensions(None, Some(h))) {
                            println!("{}", e);
                        }
                    } else {
                        println!("Unable to borrow field as text");
                    }
                } else {
                    println!("Unable to borrow plotview");
                }
                glib::signal::Inhibit(true)
            });
        }*/
        
        {
            let plot_view = plot_view.clone();
            dim_combo.connect_changed(move |combo| {
                let txt = combo.get_active_text();
                if let Ok(mut pl) = plot_view.try_borrow_mut() {
                    if let Some(txt) = combo.get_active_text() {
                        let txt_str : String = txt.into();
                        let mut dims = txt_str.split("x").map(|s| { 
                            let st = s.trim(); 
                            let d : Option<usize> = st.parse().ok();
                            d
                        });
                        let width = if let Some(Some(dim)) = dims.next() {
                            dim
                        } else {
                            // glib.set_active_id(None);
                            return;        
                        };
                        let height = if let Some(Some(dim)) = dims.next() {
                            dim
                        } else {
                            return;        
                        }; 
                        if let Err(e) = pl.update(&mut UpdateContent::Dimensions(Some(width), Some(height))) {
                            println!("{}", e);
                        }
                    } else {
                        println!("Unable take text from combo");
                    }
                } else {
                    println!("Unable to borrow plotview");
                }
                // glib::signal::Inhibit(true)
            });
        }
        
        LayoutWindow {
            layout_list_box,
            toggles,
            open_btn,
            save_btn,
            clear_btn,
            // query_btn,
            xml_save_dialog,
            xml_load_dialog,
            file_combo,
            recent,
            horiz_ar_scale,
            vert_ar_scale,
            dim_combo,
            mapping_tree,
            win,
            layout_stack
            // layout_width_entry,
            // layout_height_entry
        }
    }

    pub fn reset(&self, split : GroupSplit) {
        let toggle = &self.toggles[&split];
        if !toggle.get_active() {
            toggle.set_active(true);
        }
        for (key, toggle) in self.toggles.iter() {
            if *key != split {
                toggle.set_active(false);
            }
        }
    }

    pub fn connect_clear(&self, ws : &PlotWorkspace) {
        let ws = ws.clone();
        let file_combo = self.file_combo.clone();
        let toggles = self.toggles.clone();
        self.clear_btn.connect_clicked(move |btn| {
            ws.clear();
            file_combo.set_active_iter(None);
            toggles[&GroupSplit::Unique].set_active(true);
        });
    }

    pub fn connect_layout_load(
        glade_def : Rc<HashMap<String, String>>,
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        // plot_popover : PlotPopover,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        sources : Rc<RefCell<Vec<DataSource>>>,
        design_menu : DesignMenu,
        // scale_menus : (ScaleMenu, ScaleMenu),
        plot_toggle : ToggleButton,
        layout_window : LayoutWindow,
        layout_path : Rc<RefCell<Option<String>>>,
        ar_scales : (Scale, Scale),
        group_toolbar : GroupToolbar,
        mapping_tree : MappingTree,
    ) {
        {
            let open_btn = layout_window.open_btn.clone();
            let xml_load_dialog = layout_window.xml_load_dialog.clone();
            let plot_view = plot_view.clone();
            open_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
                if let Ok(pl) = plot_view.try_borrow() {
                    pl.parent.queue_draw();
                } else {
                    println!("Unable to borrow plot view");
                }
            });
        }

        {
            let plot_view = plot_view.clone();
            let layout_path = layout_path.clone();
            // let mapping_menus = mapping_menus.clone();
            let design_menu = design_menu.clone();
            // let scale_menus = mapping_tree.scale_menus.clone();
            // let plot_popover = plot_popover.clone();
            let glade_def = glade_def.clone();
            let table_env = table_env.clone();
            let tbl_nb = tbl_nb.clone();
            let status_stack = status_stack.clone();
            let layout_window = layout_window.clone();
            let plot_toggle = plot_toggle.clone();
            let ar_scales = ar_scales.clone();
            let group_toolbar = group_toolbar.clone();
            let mapping_tree = mapping_tree.clone();
            let sources = sources.clone();
            // TODO must not emit this changed when the combo is set by some reason other than
            // the user pressing it.
            layout_window.file_combo.clone().connect_changed(move |combo| {
                let combo_txt = combo.clone().downcast::<ComboBoxText>().unwrap();
                let opt_path_str = combo_txt.get_active_text()
                    .map(|s| s.as_str().to_string() );
                println!("Active text = {:?}", opt_path_str);
                let path_str = match opt_path_str {
                    Some(path) => {
                        // Only accept changes from a user-derived action (i.e. not pointing
                        // to "clean" layouts shipped with queries)
                        if !path.starts_with("assets/plot_layout/layout-") {
                            path
                        } else {
                            return;
                        }
                    },
                    None => return
                };
                let load_ok = Self::load_layout(
                    plot_view.clone(),
                    layout_path.clone(),
                    path_str.clone(),
                    // mapping_menus.clone(),
                    sources.clone(),
                    design_menu.clone(),
                    // scale_menus.clone(),
                    // plot_popover.clone(),
                    glade_def.clone(),
                    table_env.clone(),
                    tbl_nb.clone(),
                    status_stack.clone(),
                    layout_window.clone(),
                    plot_toggle.clone(),
                    ar_scales.clone(),
                    group_toolbar.clone(),
                    mapping_tree.clone(),
                );
                if !load_ok {
                    println!("Error loading layout");
                }
            });
        }

        {
            let recent = layout_window.recent.clone();
            layout_window.xml_load_dialog.clone().connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            let path_str = path.to_str().unwrap_or("").to_string();
                            let load_ok = Self::load_layout(
                                plot_view.clone(),
                                layout_path.clone(),
                                path_str.clone(),
                                sources.clone(),
                                // mapping_menus.clone(),
                                design_menu.clone(),
                                // scale_menus.clone(),
                                // plot_popover.clone(),
                                glade_def.clone(),
                                table_env.clone(),
                                tbl_nb.clone(),
                                status_stack.clone(),
                                layout_window.clone(),
                                plot_toggle.clone(),
                                ar_scales.clone(),
                                group_toolbar.clone(),
                                mapping_tree.clone(),
                            );
                            if load_ok {
                                recent.push_recent(path_str.clone());
                                Self::update_recent_paths(
                                    layout_window.file_combo.clone(),
                                    &recent,
                                    layout_path.clone()
                                );
                            } else {
                                println!("Failed at loadig layout. Widgets will not be updated");
                            }
                        } else {
                            println!("Could not get filename from dialog");
                        }
                    },
                    _ => { }
                }
            });
        }
    }

    fn load_layout(
        plot_view : Rc<RefCell<PlotView>>,
        layout_path : Rc<RefCell<Option<String>>>,
        string_path : String,
        // mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        sources : Rc<RefCell<Vec<DataSource>>>,
        design_menu : DesignMenu,
        // scale_menus : (ScaleMenu, ScaleMenu),
        // plot_popover : PlotPopover,
        glade_def : Rc<HashMap<String, String>>,
        table_env : Rc<RefCell<TableEnvironment>>,
        tbl_nb : TableNotebook,
        status_stack : StatusStack,
        layout_window : LayoutWindow,
        plot_toggle : ToggleButton,
        ar_scales : (Scale, Scale),
        group_toolbar : GroupToolbar,
        mapping_tree : MappingTree
    ) -> bool {
        let update_ok = match plot_view.try_borrow_mut() {
            Ok(mut pl) => {
                match pl.plot_group.load_layout(string_path.clone()) {
                    Ok(_) => {
                        layout_window.reset(pl.group_split());
                        group_toolbar.set_active_default(Some(pl.group_split()));
                        if let Ok(mut layout_path) = layout_path.try_borrow_mut() {
                            *layout_path = Some(string_path);
                            true
                        } else {
                            println!("Unable to borrow layout path mutably");
                            false
                        }
                    },
                    Err(e) => { println!("Unable to load layout: {}", e); false }
                }
            },
            Err(_) => { println!("Could not get mutable reference to Plot widget"); false }
        };
        if update_ok {
            println!("Updating mapping sources");
            mapping_tree.repopulate(plot_view.clone());
            if let Ok(mut sources) = sources.try_borrow_mut() {
                sources.clear();
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    let n_plots = pl_view.n_plots();
                    for plot_ix in 0..n_plots {
                        pl_view.change_active_area(plot_ix);
                        let new_info =  pl_view.mapping_info();
                        if let Ok(t_env) = table_env.try_borrow() {
                            for (name, ty, props) in new_info.iter() {
                                let mut source : DataSource = Default::default();
                                source.name = name.to_string();
                                source.ty = ty.to_string();
                                source.plot_ix = plot_ix;
                                source.hist_ix = t_env.current_hist_index();
                                PlotWorkspace::update_source(&mut source, tbl_nb.full_selected_cols(), &t_env)
                                    .map_err(|e| format!("{}", e) );
                                PlotWorkspace::update_data(&source, &t_env, &mut pl_view)
                                    .map_err(|e| format!("{}", e) );
                                sources.push(source);
                            }
                        } else {
                            println!("Unable to borrow table environment");
                        }
                    }
                } else {
                    println!("Unable to borrow plot view");
                }
            } else {
                println!("Unable to borrow sources vector");
            }
            println!("Updating layout widgets");
            Self::update_layout_widgets(
                design_menu.clone(),
                mapping_tree.scale_menus.clone(),
                plot_view.clone(),
                ar_scales
            );
            println!("Layout widgets saved");
            status_stack.try_show_alt();
            plot_toggle.set_active(true);
            if let Ok(pl_view) = plot_view.try_borrow() {
                pl_view.parent.queue_draw();
            } else {
                println!("Unable to get reference to plot view");
            }
        }
        update_ok
    }

    fn update_layout_widgets(
        design_menu : DesignMenu,
        scale_menus : (ScaleMenu, ScaleMenu),
        plot_view : Rc<RefCell<PlotView>>,
        ar_scales : (Scale, Scale)
    ) {
        let (design, info_x, info_y, ar) = match plot_view.try_borrow() {
            Ok(pl) => {
                let design = pl.plot_group.design_info();
                let info_x = pl.current_scale_info("x");
                let info_y = pl.current_scale_info("y");
                let ar = pl.aspect_ratio();
                (design, info_x, info_y, ar)
            },
            _ => {
                println!("Could not fetch plotview reference to update layout");
                return;
            }
        };

        // It is important to call those updates outside the plot_view borrow because those updates
        // will trigger the scale_set, entry_set, etc. signals inside each menu, which
        // assume plot_view can be borrowed mutably.
        design_menu.update(design);
        scale_menus.0.update(info_x);
        scale_menus.1.update(info_y);
        ar_scales.0.set_value(ar.0);
        ar_scales.1.set_value(ar.1);
    }

}


pub mod table_widget;

pub mod table_notebook;

pub mod status_stack;

pub mod sql_popover;

pub mod functions;

pub mod utils {

    use std::env;
    use gtk::*;
    use gio::prelude::*;

    fn exec_dir() -> Result<String, &'static str> {
        let exe_path = env::current_exe().map_err(|_| "Could not get executable path")?;
        let exe_dir = exe_path.as_path().parent().ok_or("CLI executable has no parent dir")?
            .to_str().ok_or("Could not convert path to str")?;
        Ok(exe_dir.to_string())
    }

    pub fn glade_path(filename : &str) -> Result<String, &'static str> {
        let exe_dir = exec_dir()?;
        let path = exe_dir + "/../../assets/gui/" + filename;
        Ok(path)
    }

    pub fn provider_from_path(filename : &str) -> Result<CssProvider, &'static str> {
        let provider =  CssProvider::new();
        let exe_dir = exec_dir()?;
        let path = exe_dir + "/../../assets/styles/" + filename;
        println!("{}", path);
        provider.load_from_path(&path[..]).map_err(|_| "Unable to load Css provider")?;
        Ok(provider)
    }

}



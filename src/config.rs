use std::{env, path::PathBuf};

use lazy_static::lazy_static;

fn get_plugin_directory() -> PathBuf {
    if let Ok(path) = env::var("LME_PLUGINS_DIR") {
        PathBuf::from(path)
    } else {
        env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("plugins")
    }
}

lazy_static! {
    pub static ref PLUGIN_DIRECTORY: PathBuf = get_plugin_directory();
}

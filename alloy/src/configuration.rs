//! There are two files that store properties for Emulsion, the *cache* and the *config*.
//!
//! The most important distinction between these is that Emulsion never writes to the *config*
//! but it does write to the *cache* to save portions of the state of the program (e.g. window size
//! and position).
//!
//! Furthermore it's generally true that the user will only edit the *config* to specify their
//! preferences.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
}
impl Theme {
    pub fn switch_theme(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Antialias {
    Auto,
    Always,
    Never,
}
impl Default for Antialias {
    fn default() -> Self {
        Antialias::Auto
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CacheImageSection {
    pub fit_stretches: bool,
    pub antialiasing: Antialias,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize)]
pub struct ConfigImageSection {
    pub antialiasing: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CacheWindowSection {
    pub dark: bool,
    pub win_w: u32,
    pub win_h: u32,
    pub win_x: i32,
    pub win_y: i32,
}
impl Default for CacheWindowSection {
    fn default() -> Self {
        Self {
            dark: false,
            win_w: 580,
            win_h: 558,
            win_x: 64,
            win_y: 64,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigWindowSection {
    pub start_fullscreen: Option<bool>,
    pub start_maximized: Option<bool>,
    pub show_bottom_bar: Option<bool>,
    pub theme: Option<Theme>,
    pub use_last_window_area: Option<bool>,
    pub win_w: Option<u32>,
    pub win_h: Option<u32>,
    pub win_x: Option<i32>,
    pub win_y: Option<i32>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct ConfigUpdateSection {
    pub check_updates: bool,
}
impl Default for ConfigUpdateSection {
    fn default() -> Self {
        Self {
            check_updates: true,
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CacheUpdateSection {
    pub last_checked: u64,
}

impl CacheUpdateSection {
    pub fn update_check_needed(&self) -> bool {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH + Duration::from_secs(self.last_checked))
            .unwrap_or_else(|_| Duration::from_secs(0));

        duration > Duration::from_secs(60 * 60 * 24) // 24 hours
    }

    pub fn set_update_check_time(&mut self) {
        self.last_checked = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();
    }
}

#[derive(Deserialize)]
struct IncompleteCache {
    pub window: Option<CacheWindowSection>,
    pub updates: Option<CacheUpdateSection>,
    pub image: Option<CacheImageSection>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize)]
pub struct Cache {
    pub window: CacheWindowSection,
    pub updates: CacheUpdateSection,
    pub image: CacheImageSection,
}
impl From<IncompleteCache> for Cache {
    fn from(cache: IncompleteCache) -> Self {
        Self {
            window: cache.window.unwrap_or_default(),
            updates: cache.updates.unwrap_or_default(),
            image: cache.image.unwrap_or_default(),
        }
    }
}
impl Cache {
    pub fn theme(&self) -> Theme {
        if self.window.dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.window.dark = theme == Theme::Dark;
    }

    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<Cache, String> {
        let file_path = file_path.as_ref();
        let cfg_str = fs::read_to_string(file_path).map_err(|_| {
            format!("Could not read cache from {:?}", file_path)
        })?;
        let result: IncompleteCache =
            toml::from_str(&cfg_str).map_err(|e| format!("{}", e))?;
        //println!("Read cache from file:\n{:#?}", result);
        Ok(result.into())
    }

    pub fn save<P: AsRef<Path>>(&self, file_path: P) -> Result<(), String> {
        let file_path = file_path.as_ref();
        let string = toml::to_string(self).map_err(|e| format!("{}", e))?;
        fs::write(file_path, string).map_err(|_| {
            format!("Could not write to cache file {:?}", file_path)
        })?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize)]
pub struct Command {
    pub input: Vec<String>,
    pub program: String,
    pub args: Option<Vec<String>>,
    pub envs: Option<Vec<EnvVar>>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize)]
pub struct TitleSection {
    pub displayed_folders: Option<u32>,
    pub show_program_name: Option<bool>,
}
impl TitleSection {
    pub fn format_file_path<'a>(&self, file_path: &'a Path) -> Cow<'a, str> {
        match self.displayed_folders {
            Some(0) | None => file_path.file_name().unwrap().to_string_lossy(),
            Some(n) => {
                let mut component_count = 0;
                // On Windows the root can be the second component, when a `Prefix` is the first.
                let mut root_index = 0;
                for (idx, c) in file_path.components().enumerate() {
                    component_count += 1;
                    if c == std::path::Component::RootDir {
                        root_index = idx as u32;
                    }
                }
                let path = if (component_count - root_index) <= (1 + n) {
                    file_path
                        .to_string_lossy()
                        .trim_start_matches("\\\\?\\")
                        .to_owned()
                        .into()
                } else {
                    let ancestor = file_path
                        .ancestors()
                        .take(2 + n as usize)
                        .last()
                        .unwrap();
                    file_path.strip_prefix(ancestor).unwrap().to_string_lossy()
                };
                path
            }
        }
    }

    pub fn format_program_name(&self) -> &'static str {
        match self.show_program_name {
            Some(false) => "",
            _ => " : E M U L S I O N",
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Configuration {
    pub bindings: Option<BTreeMap<String, Vec<String>>>,
    pub commands: Option<Vec<Command>>,
    pub updates: Option<ConfigUpdateSection>,
    pub title: Option<TitleSection>,
    pub image: Option<ConfigImageSection>,
    pub window: Option<ConfigWindowSection>,
}
impl Configuration {
    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<Configuration, String> {
        let file_path = file_path.as_ref();
        let cfg_str = fs::read_to_string(file_path).map_err(|_| {
            format!("Could not read config from {:?}", file_path)
        })?;
        let result =
            toml::from_str(cfg_str.as_ref()).map_err(|e| format!("{}", e))?;
        //println!("Read config from file:\n{:#?}", result);
        Ok(result)
    }
}

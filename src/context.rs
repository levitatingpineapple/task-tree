use chrono_tz::Tz;
use serde::Deserialize;
use std::{fs::read_to_string, path::PathBuf};
use tokio::sync::OnceCell;

static CONTEXT: OnceCell<Context> = OnceCell::const_new();

pub fn set(workspace: &PathBuf) -> Result<(), ContextErr> {
    let string = read_to_string(workspace.join(".task-tree.toml"))?;
    CONTEXT
        .set(Context {
            config: toml::from_str(&string)?,
            workspace: workspace.clone(),
        })
        .expect("Context is only set once");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ContextErr {
    #[error("Io: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config decode: {0}")]
    Toml(#[from] toml::de::Error),
}

#[cfg(not(test))]
pub fn get() -> &'static Context {
    CONTEXT.get().expect("Context has been initialised")
}

#[cfg(test)]
pub fn get() -> &'static Context {
    static TEST_CONTEXT: std::sync::OnceLock<Context> = std::sync::OnceLock::new();
    TEST_CONTEXT.get_or_init(|| Context::dummy())
}

#[derive(Debug)]
pub struct Context {
    config: Config,
    workspace: PathBuf,
}

impl Context {
    pub fn todo(&self) -> PathBuf {
        self.workspace.join(&self.config.paths.todo)
    }

    pub fn done(&self) -> PathBuf {
        self.workspace.join(&self.config.paths.done)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn enabled(&self, url: &reqwest::Url) -> bool {
        [self.todo(), self.done()].contains(&url.to_file_path().expect("Local file"))
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub caldav: CalDAV,
    #[serde(default)]
    paths: Paths,
    pub timezone: Tz,
}

#[derive(Deserialize, Debug)]
pub struct CalDAV {
    pub url: String,
    pub user: String,
    pub pass: String,
}

#[derive(Deserialize, Debug)]
struct Paths {
    todo: PathBuf,
    done: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            todo: PathBuf::from("todo.md"),
            done: PathBuf::from("done.md"),
        }
    }
}

#[cfg(test)]
impl Context {
    pub fn dummy() -> Self {
        Context {
            config: Config::dummy(),
            workspace: PathBuf::from("/tmp/test_workspace"),
        }
    }
}

#[cfg(test)]
impl Config {
    pub fn dummy() -> Self {
        Config {
            caldav: CalDAV::dummy(),
            paths: Paths::default(),
            timezone: chrono_tz::America::Santiago,
        }
    }
}

#[cfg(test)]
impl CalDAV {
    pub fn dummy() -> Self {
        CalDAV {
            url: "https://example.com/caldav".to_string(),
            user: "test_user".to_string(),
            pass: "test_pass".to_string(),
        }
    }
}

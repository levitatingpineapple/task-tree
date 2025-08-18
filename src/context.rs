use chrono_tz::Tz;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Context {
    config: Config,
    workspace: PathBuf,
}

impl Context {
    pub fn new(config: Config, workspace: PathBuf) -> Self {
        Context { config, workspace }
    }

    pub fn todo(&self) -> PathBuf {
        self.workspace.join(&self.config.paths.todo)
    }

    pub fn done(&self) -> PathBuf {
        self.workspace.join(&self.config.paths.done)
    }

    pub fn config(&self) -> &Config {
        &self.config
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

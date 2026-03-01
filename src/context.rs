use chrono_tz::Tz;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::OnceCell;

static CONTEXT: OnceCell<Context> = OnceCell::const_new();

pub fn set(context: Context) {
    CONTEXT.set(context).expect("Context is only set once");
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

// pub fn init_context(workspace: PathBuf) -> jsonrpc::Result<()> {
//     let config: Config = fs::read_to_string(&workspace.join(".task-tree.toml"))
//         .map_err(|_| jsonrpc::Error::invalid_params("Failed to find .task-tree.toml file"))
//         .and_then(|content| {
//             toml::from_str(&content).map_err(|e| {
//                 jsonrpc::Error::invalid_params(format!("Failed to parse .task-tree.toml: {}", e))
//             })
//         })?;
//     CONTEXT
//         .set(Context::new(config, workspace))
//         .expect("Init should be called only once");
//     Ok(())
// }

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

mod chart;
mod commands;
mod context;
mod group;
mod lsp;
mod ranged_err;
mod session;
mod task;
mod tasktree;
mod tree;

use crate::{
    lsp::TaskTreeServer,
    session::Session,
    task::Task,
    tasktree::{TaskPath, TaskTree},
    tree::Parent,
};
use chrono::{Duration, TimeZone, Utc};
use clap::{CommandFactory, Parser};
use clap_complete::{aot::Fish, generate};
use colored::{Color, Colorize};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::{
    env::current_dir,
    fmt::{self, Display},
    fs,
    process::exit,
    str::FromStr,
};
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};

#[derive(Parser)]
#[command(author, version, about)]
enum Args {
    /// Start the language server
    Lsp,
    /// Open a chart in a browser
    Chart,
    /// List task paths
    List,
    /// Start an active session now
    Start { task_path: TaskPath },
    /// Status of current session
    Status,
    /// Stop active session
    Stop,
    /// Generate fish shell autocompletions
    Generate,
}

#[tokio::main]
async fn main() {
    match Args::parse() {
        Args::Lsp => {
            let (service, socket) = LspService::new(|client| TaskTreeServer::new(client));
            Server::new(stdin(), stdout(), socket).serve(service).await;
        }
        Args::Chart => {
            set_context_from_pwd();
            chart::serve().await;
        }
        Args::List => {
            set_context_from_pwd();
            commands::list_task_paths();
        }

        Args::Start { task_path } => {
            set_context_from_pwd();
            let session_path = context::get().active_session();
            if session_path.exists() {
                red_panic("Session already active!");
            }
            let toml_str = toml::to_string(&Active::new(task_path)).expect("Valid toml");
            fs::write(session_path, toml_str).expect("Write to active session file");
        }

        Args::Status => {
            set_context_from_pwd();
            let session_path = context::get().active_session();
            if let Ok(toml_str) = fs::read_to_string(&session_path) {
                let active: Active = toml::from_str(&toml_str).expect("Valid toml");
                println!("\n{}\n", "Running:".green());
                print!("{active}\n");
            } else {
                println!("Nothing is running")
            }
        }

        Args::Stop => {
            set_context_from_pwd();
            let session_path = context::get().active_session();
            let toml_str = fs::read_to_string(&session_path).e("Session not active!");
            let active: Active = toml::from_str(&toml_str).expect("Valid toml");
            match active.session() {
                Some(session) => {
                    let todo_path = context::get().todo();
                    let todo_str = &fs::read_to_string(&todo_path).expect("Missing todo file");
                    let mut tree = TaskTree::from_str(todo_str).expect("Valid tree");
                    let gp = active.task_path.group.clone();
                    let group = tree.get_mut(gp).expect("Group exists");
                    let tp = active.task_path.task.clone();
                    let task: &mut Task = group.get_mut(tp).expect("Task exists");
                    task.sessions.push(session);
                    fs::write(&todo_path, tree.to_string()).expect("Write todo file");
                    fs::remove_file(&session_path).expect("Remove active session file");
                    println!("\n{}\n", "Stopped:".red());
                    print!("{active}\n");
                }
                None => {
                    fs::remove_file(&session_path).expect("Remove active session file");
                    red_panic("Empty Session")
                }
            };
        }

        Args::Generate => {
            generate(
                Fish,
                &mut Args::command(),
                "task-tree",
                &mut std::io::stdout(),
            );
            // Generate dynamic autocomplete for `start` command
            println!(
                "complete -c task-tree -n '__fish_seen_subcommand_from start' -a '(task-tree list)'"
            )
        }
    }
}

// MARK: Error helpers

fn red_panic(reason: &str) -> ! {
    eprintln!("{}", reason.red().bold());
    exit(1)
}

pub trait RedExpect<T> {
    fn e(self, message: &str) -> T;
}

impl<T, E: Display> RedExpect<T> for Result<T, E> {
    fn e(self, message: &str) -> T {
        match self {
            Ok(val) => val,
            Err(err) => {
                eprintln!("{}", message.red().bold());
                eprintln!("{}", err.to_string().white());
                exit(1)
            }
        }
    }
}

impl<T> RedExpect<T> for Option<T> {
    fn e(self, message: &str) -> T {
        match self {
            Some(val) => val,
            None => red_panic(message),
        }
    }
}

fn set_context_from_pwd() {
    let pwd = current_dir().expect("Valid directory");
    context::set(&pwd).e("No config file in current directory");
}

/// Currently active session, stored as toml
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Active {
    #[serde_as(as = "DisplayFromStr")]
    pub task_path: TaskPath,
    pub start: i64,
}

impl Active {
    fn new(task_path: TaskPath) -> Self {
        Self {
            task_path,
            start: Utc::now().timestamp(),
        }
    }

    fn session(&self) -> Option<Session> {
        Session::from_utc(
            context::get().config().timezone,
            Utc.timestamp_opt(self.start, 0).unwrap(),
            Utc::now(),
            Duration::minutes(1),
        )
        .ok()
    }
}

impl Display for Active {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {}",
            "Zone:".white(),
            self.task_path
                .group
                .to_string()
                .color_char('/', Color::White)
                .bright_green()
        )?;
        writeln!(
            f,
            "{} {}",
            "Task:".white(),
            self.task_path
                .task
                .to_string()
                .color_char('/', Color::White)
                .bright_blue()
        )?;
        write!(f, "{}", "Sesh: ".white())?;
        match self.session() {
            Some(session) => {
                write!(
                    f,
                    "{} ",
                    session
                        .to_string()
                        .color_char('-', Color::Red)
                        .color_char('_', Color::White)
                        .color_char(':', Color::White)
                        .color_char('/', Color::White)
                        .bright_magenta()
                )?;
                writeln!(
                    f,
                    "{}",
                    hours_minutes(session.range.into_dt_span().duration())
                )?;
            }
            None => writeln!(f, "{}", "Empty".red())?,
        }
        Ok(())
    }
}

fn hours_minutes(duration: Duration) -> String {
    let total_minutes = duration.num_seconds() / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours > 0 {
        format!(
            "{}{}{}{}{}",
            "(".white(),
            hours.to_string().bright_red(),
            "h ".white(),
            minutes.to_string().bright_red(),
            "m)".white()
        )
    } else {
        format!("{}m", minutes.to_string().red())
    }
}

// MARK: Colored characters

pub trait ColoredChar {
    fn color_char(&self, target: char, color: Color) -> String;
}

impl ColoredChar for String {
    fn color_char(&self, target: char, color: Color) -> String {
        self.chars()
            .map(|c| {
                if c == target {
                    c.to_string().color(color).to_string()
                } else {
                    c.to_string()
                }
            })
            .collect()
    }
}

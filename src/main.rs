mod chart;
mod commands;
mod context;
mod group;
mod lsp;
mod print_color;
mod ranged_err;
mod session;
mod task;
mod tasktree;
mod tree;

use crate::{
    lsp::TaskTreeServer,
    print_color::{StringExt, rounded_box},
    session::Session,
    task::Task,
    tasktree::{TaskPath, TaskTree},
    tree::Parent,
};
use chrono::{Duration, TimeZone, Utc};
use clap::{CommandFactory, Parser, Subcommand};
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
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the language server
    Lsp,
    /// Open a chart in a browser
    Chart,
    /// Start an active session now
    Now { task_path: TaskPath },
    /// End active session
    End,
    /// Shell autocompletions
    Complete {
        #[command(subcommand)]
        completions: Completions,
    },
}

#[derive(Subcommand)]
enum Completions {
    /// List task paths
    List,
    /// Generate completions for fish shell
    FishGenerate,
}

#[tokio::main]
async fn main() {
    match Args::parse().command {
        Some(command) => match command {
            Command::Lsp => {
                let (service, socket) = LspService::new(|client| TaskTreeServer::new(client));
                Server::new(stdin(), stdout(), socket).serve(service).await;
            }
            Command::Chart => {
                set_context_from_pwd();
                chart::serve().await;
            }

            Command::Now { task_path } => {
                set_context_from_pwd();
                let session_path = context::get().active_session();
                if session_path.exists() {
                    red_panic("Session already active!");
                }
                let active = Active::new(task_path);
                let toml_str = toml::to_string(&active).expect("Valid toml");
                fs::write(session_path, toml_str).expect("Write to active session file");
                println!(
                    "{}",
                    rounded_box(
                        "Started",
                        active,
                        Some(Color::Magenta),
                        Some(Color::BrightMagenta),
                    )
                );
            }

            Command::End => {
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
                        println!(
                            "{}",
                            rounded_box(
                                "Stopped",
                                active,
                                Some(Color::Red),
                                Some(Color::BrightRed)
                            )
                        );
                    }
                    None => {
                        fs::remove_file(&session_path).expect("Remove active session file");
                        println!(
                            "{}",
                            rounded_box(
                                "Discarded",
                                active,
                                Some(Color::Red),
                                Some(Color::BrightRed)
                            )
                        );
                    }
                };
            }

            Command::Complete { completions } => match completions {
                Completions::List => {
                    set_context_from_pwd();
                    commands::print_incomplete_task_paths();
                }
                Completions::FishGenerate => {
                    generate(Fish, &mut Args::command(), "tt", &mut std::io::stdout());
                    println!(
                        "complete -c tt -n '__fish_seen_subcommand_from now' -a '(tt complete list)'"
                    )
                }
            },
        },
        None => {
            set_context_from_pwd();
            let session_path = context::get().active_session();
            if let Ok(toml_str) = fs::read_to_string(&session_path) {
                let active: Active = toml::from_str(&toml_str).expect("Valid toml");
                println!(
                    "{}",
                    rounded_box(
                        "Running",
                        active,
                        Some(Color::Green),
                        Some(Color::BrightGreen)
                    )
                );
            } else {
                println!(
                    "{}",
                    rounded_box(
                        "Nothing is running",
                        format!(
                            "{} {} {}",
                            "Run".white(),
                            "tt help".yellow(),
                            "for options.".white()
                        ),
                        Some(Color::White),
                        Some(Color::White)
                    )
                );
            }
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
    let workspace = std::env::var("TASK_TREE_DIR")
        .ok()
        .map(|ttd| std::path::PathBuf::from(ttd))
        .unwrap_or(current_dir().e("Valid directory"));
    context::set(&workspace).e("No config file in directory");
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
        let zone = self
            .task_path
            .group
            .to_string()
            .color_char('/', Color::White)
            .bright_green();
        let task = self
            .task_path
            .task
            .to_string()
            .color_char('/', Color::White)
            .bright_blue();
        let sesh = match self.session() {
            Some(s) => format!(
                "{} {}",
                s.to_string()
                    .color_char('-', Color::Red)
                    .color_char('_', Color::White)
                    .color_char(':', Color::White)
                    .color_char('/', Color::White)
                    .bright_magenta(),
                hours_minutes(s.range.into_dt_span().duration())
            ),
            None => "Empty".red().to_string(),
        };
        writeln!(f, "{} {zone}", "Zone:".white())?;
        writeln!(f, "{} {task}", "Task:".white())?;
        writeln!(f, "{} {sesh}", "Sesh:".white())
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

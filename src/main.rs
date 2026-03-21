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

use clap::Parser;
use std::path::PathBuf;
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};

#[derive(Parser)]
#[command(author, version, about)]
enum Args {
    Lsp,
    Chart {
        path: PathBuf,
    },
    List,
    #[command(short_flag = 's')]
    Start {
        task: crate::tasktree::TaskPath,
    },
    #[command(short_flag = 'e')]
    End,
}

#[tokio::main]
async fn main() {
    match Args::parse() {
        Args::Lsp => {
            let (service, socket) = LspService::new(|client| lsp::TaskTreeServer::new(client));
            Server::new(stdin(), stdout(), socket).serve(service).await;
        }
        Args::Chart { path } => {
            context::set(&path).expect("Valid config");
            chart::serve().await;
        }
        Args::List => {
            let pwd = std::env::current_dir().expect("Valid directory");
            context::set(&pwd).expect("Valid config");
            commands::print_autocomplete();
        }
        Args::Start { task } => {
            let pwd = std::env::current_dir().expect("Valid directory");
            context::set(&pwd).expect("Valid config");
            let session_path = context::get().active_session();
            if session_path.exists() {
                println!("\x1b[31mSession already active\x1b[0m");
                return;
            }
            let active = active::Active {
                task,
                start: chrono::Utc::now().timestamp(),
            };
            let toml_str = toml::to_string(&active).expect("Valid toml");
            std::fs::write(session_path, toml_str).expect("Write to active session file");
        }
        Args::End => {
            let pwd = std::env::current_dir().expect("Valid directory");
            context::set(&pwd).expect("Valid config");
            let session_path = context::get().active_session();
            if !session_path.exists() {
                println!("\x1b[31mNo active session\x1b[0m");
                return;
            }
            let toml_str =
                std::fs::read_to_string(&session_path).expect("Read active session file");
            let active: active::Active = toml::from_str(&toml_str).expect("Valid toml");

            use crate::tree::Parent;
            use std::str::FromStr;
            let todo_path = context::get().todo();
            let mut tree = crate::tasktree::TaskTree::from_str(
                &std::fs::read_to_string(&todo_path).expect("Read todo file"),
            )
            .expect("Valid tree");

            let group = tree.get_mut(active.task.group).expect("Group exists");
            let task: &mut crate::task::Task =
                group.get_mut(active.task.task).expect("Task exists");

            use chrono::TimeZone;
            let tz = context::get().config().timezone;
            let start_dt = chrono::Utc
                .timestamp_opt(active.start, 0)
                .unwrap()
                .with_timezone(&tz);
            let end_dt = chrono::Utc::now().with_timezone(&tz);
            let span = crate::session::range::Span::new(start_dt, end_dt);
            let range = crate::session::range::Range::Timed(span);
            let session = crate::session::Session {
                range,
                repeat: None,
            };

            task.sessions.push(session);

            std::fs::write(&todo_path, tree.to_string()).expect("Write todo file");
            std::fs::remove_file(&session_path).expect("Remove active session file");
        }
    }
}

mod active {
    use crate::tasktree::TaskPath;
    use serde::{Deserialize, Serialize};
    use serde_with::{DisplayFromStr, serde_as};

    #[serde_as]
    #[derive(Serialize, Deserialize)]
    pub struct Active {
        #[serde_as(as = "DisplayFromStr")]
        pub task: TaskPath,
        pub start: i64,
    }
}

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

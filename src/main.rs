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
enum Arguments {
    Init,
    Lsp,
    Chart { path: PathBuf },
}

#[tokio::main]
async fn main() {
    match Arguments::parse() {
        Arguments::Lsp => {
            let (service, socket) = LspService::new(|client| lsp::TaskTreeServer::new(client));
            Server::new(stdin(), stdout(), socket).serve(service).await;
        }
        Arguments::Chart { path } => {
            context::set(&path).unwrap();
            chart::serve().await;
        }
        Arguments::Init => todo!("Implement init in current folder"),
    }
}

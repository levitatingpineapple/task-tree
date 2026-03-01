mod chart;
mod commands;
mod context;
mod group;
mod ranged_err;
mod server;
mod session;
mod task;
mod tasktree;
mod tree;

use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};

// TODO: Add clap with lsp, chart and init subcommands

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| server::TaskTreeServer::new(client));
    Server::new(stdin(), stdout(), socket).serve(service).await;
}

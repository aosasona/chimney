mod config;
mod error;
mod server;

mod commands;
mod log_macros;

#[tokio::main]
async fn main() {
    let command = commands::parse_args();

    if let Err(error) = command.run().await {
        log_error!(error);
        return;
    };
}

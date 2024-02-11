mod commands;
mod config;
mod error;
mod server;

#[tokio::main]
async fn main() {
    let command = commands::parse_args();

    if let Err(error) = command.run().await {
        eprintln!("\x1b[91m[Error] {}\x1b[0m", error);
        return;
    };
}

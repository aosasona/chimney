mod commands;
mod config;
mod error;
mod server;

fn main() {
    let command = commands::parse_args();

    if let Err(error) = command.run() {
        eprintln!("\x1b[91m[Error] {}\x1b[0m", error);
        return;
    };
}

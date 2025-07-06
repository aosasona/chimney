pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod format;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::new();

    if let Err(e) = cli.execute().await {
        dbg!(&e);
        log::error!("Error: {e}");
        std::process::exit(1);
    }
}

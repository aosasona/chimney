pub(crate) mod cli;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::new();

    if let Err(e) = cli.run().await {
        dbg!(&e);
        log::error!("Error: {}", e);
        std::process::exit(1);
    }
}

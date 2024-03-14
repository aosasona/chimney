use chimney::{commands, log_error};

#[tokio::main]
async fn main() {
    let command = commands::parse_args();

    if let Err(error) = command.run().await {
        log_error!(error);
        return;
    };
}

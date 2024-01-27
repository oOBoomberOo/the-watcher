use the_watcher::repl::{self, Repl, ReplError};

#[tokio::main]
async fn main() -> Result<(), ReplError> {
    tracing_subscriber::fmt().pretty().init();
    let mut repl = Repl::new()?;

    if let Err(err) = repl::start(&mut repl).await {
        eprintln!("{}", err);
        std::process::exit(1);
    }

    Ok(())
}

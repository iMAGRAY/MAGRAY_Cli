use clap::Parser;

/// CLI entry point
#[derive(Parser, Debug)]
#[command(name = "ourcli")]
enum Cmd {
    Init,
    Run { goal: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = Cmd::parse();
    match cmd {
        Cmd::Init => println!("init project docstore stub"),
        Cmd::Run { goal } => println!("run goal: {}", goal),
    }
    Ok(())
}

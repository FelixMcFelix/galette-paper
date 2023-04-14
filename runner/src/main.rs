use clap::Parser;
use runner::{
	cli::{Cli, Commands},
	subprograms,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let config = Cli::parse();

	match config.command {
		Commands::MakeKeys(mk) => subprograms::make_keys(mk).await,
		Commands::Run(r) => subprograms::run(r).await,
	}?;

	Ok(())
}

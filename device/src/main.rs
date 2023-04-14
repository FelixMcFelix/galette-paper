use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cfg = device::cli::Cli::parse();

	device::run(cfg).await?;

	Ok(())
}

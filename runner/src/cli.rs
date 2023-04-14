use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	MakeKeys(MakeKeys),
	Run(Run),
}

#[derive(Args)]
pub struct MakeKeys {
	#[clap(default_value_t = String::from("keys"), value_parser, long)]
	pub key_dir: String,
	#[clap(default_value_t = String::from("machines.toml"), value_parser, long)]
	pub def_file: String,
}

#[derive(Args)]
pub struct Run {
	#[clap(default_value_t = String::from("keys"), value_parser, long)]
	pub key_dir: String,
	#[clap(default_value_t = String::from("machines.toml"), value_parser, long)]
	pub def_file: String,
	#[clap(default_value_t = String::from("instances"), value_parser, long)]
	pub instances_dir: String,
	#[clap(default_value_t = String::from("pktgen"), value_parser, long)]
	pub pktgen_name: String,

	#[clap(long)]
	/// Enable this flag if DUT and pktgen machines are connected directly rather than via switch.
	pub back_to_back: bool,

	pub experiments: Vec<String>,
}

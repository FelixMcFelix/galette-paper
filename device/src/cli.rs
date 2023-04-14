use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
	#[clap(default_value_t = String::from("keys"), value_parser, long)]
	pub key_dir: String,
	#[clap(default_value_t = String::from("self"), value_parser, long)]
	pub my_name: String,
	#[clap(default_value_t = String::from("runner"), value_parser, long)]
	pub client_name: String,
}

use std::path::PathBuf;

use config::Machine;
use tokio::io::AsyncWriteExt;

use crate::cli::MakeKeys;

pub async fn make_keys(params: MakeKeys) -> anyhow::Result<()> {
	let machines = config::load_file::<Machine>(&params.def_file).await?;

	let mut write_path = PathBuf::from(params.key_dir);
	tokio::fs::create_dir_all(&write_path).await?;

	eprint!("Keys generated for...");
	let mut stderr = tokio::io::stderr();
	stderr.flush().await?;

	for machine in machines {
		let hostname = machine.hostname();
		let names: &[_] = &[
			"localhost".into(),
			hostname.into(),
			format!("{hostname}.local"),
		];
		let cert = rcgen::generate_simple_self_signed(names)?;

		write_path.push(format!("{}.cert.der", machine.hostname()));
		tokio::fs::write(&write_path, cert.serialize_der()?).await?;
		write_path.pop();

		write_path.push(format!("{}.key.der", machine.hostname()));
		tokio::fs::write(&write_path, cert.serialize_private_key_der()).await?;
		write_path.pop();

		eprint!(" {}/{}", machine.name, machine.hostname());
		stderr.flush().await?;
	}
	eprintln!();

	Ok(())
}

use std::{
	collections::HashMap,
	num::NonZeroU64,
	path::{Path, PathBuf},
	time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, process::Command as TokCommand};

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[must_use]
pub struct Sequence(pub u64);

#[derive(Debug, Deserialize, Serialize)]
pub struct Message<T> {
	pub seq: Sequence,
	pub cmd: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum C2S {
	Ping,
	Commands(Vec<Command>),
	ResetResults,
	Terminate(Sequence),
	Download(Vec<String>),
	Upload(Vec<SentFile>),
	MeasurePower(MeasurePower, Option<Sequence>),
	MeasureCpu(MeasureCpu, Option<Sequence>),
	ToggleMeasurement(Sequence),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MeasurePower {
	pub mac_address: [u8; 6],
	pub poll_interval: Duration,
	pub start_polling: bool,
	pub max_measurements: Option<NonZeroU64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MeasureCpu {
	pub poll_interval: Duration,
	pub start_polling: bool,
	pub max_measurements: Option<NonZeroU64>,
}

impl C2S {
	pub fn has_reply(&self) -> bool {
		match self {
			Self::Ping | Self::Commands(_) | Self::Download(_) | Self::MeasurePower(_, _) | Self::MeasureCpu(_, _) => true,
			_ => false,
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Command {
	pub command: Vec<Invocation>,
	pub env: HashMap<String, String>,
	pub cwd: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Invocation {
	pub prog: String,
	pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum S2C {
	Pong,
	CommandDone(Vec<(i32, Vec<u8>, Vec<u8>)>),
	Download(Vec<SentFile>),
	PowerData(Vec<PowerSample>),
	CpuData(Vec<String>),
	Error(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SentFile {
	pub filename: String,
	pub rename: Option<String>,
	pub data: Vec<u8>,
	pub class: FileClass,
}

impl SentFile {
	pub async fn load(p: impl AsRef<Path>) -> std::io::Result<Self> {
		let mut file = tokio::fs::File::open(&p).await?;
		let metadata = file.metadata().await?;

		let (data, class) = if metadata.is_dir() {
			let uuid = uuid::Uuid::new_v4();
			let filename = format!("{uuid}.tar");

			TokCommand::new("tar")
				.args(&["-C", &p.as_ref().to_str().unwrap()])
				.args(&["-c", "."])
				.args(&["-f", &filename])
				.output()
				.await?;

			let bytes = tokio::fs::read(&filename).await?;

			tokio::fs::remove_file(&filename).await?;

			(bytes, FileClass::Directory)
		} else {
			let mut my_vec = vec![];
			file.read_to_end(&mut my_vec).await?;

			(my_vec, FileClass::File)
		};

		Ok(Self {
			filename: p.as_ref().to_str().unwrap().to_string(),
			rename: None,
			data,
			class,
		})
	}

	pub async fn extract(&self, base_dir: impl AsRef<Path>) -> std::io::Result<()> {
		let mut target_file = PathBuf::from(base_dir.as_ref());

		target_file.push(self.rename.as_ref().unwrap_or(&self.filename));
		eprintln!("Writing file '{}' into '{:?}'.", self.filename, target_file);

		let mut target_dir = target_file.clone();

		match &self.class {
			FileClass::File => {
				target_dir.pop();
				tokio::fs::create_dir_all(target_dir).await?;

				tokio::fs::write(&target_file, &self.data).await
			},
			FileClass::Directory => {
				tokio::fs::create_dir_all(&target_dir).await?;

				let uuid = uuid::Uuid::new_v4();
				let filename = format!("{uuid}.tar");

				tokio::fs::write(&filename, &self.data).await?;

				TokCommand::new("tar")
					.arg("-x")
					.args(&["-C", &target_dir.to_str().unwrap()])
					.args(&["-f", &filename])
					.output()
					.await?;

				tokio::fs::remove_file(&filename).await
			},
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FileClass {
	File,
	Directory,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PowerSample {
	pub total_m_watt_hour: u32,
	pub total_m_amp_hour: u32,
	pub m_watts: u32,
	pub m_volts: u16,
	pub m_amps: u16,
}

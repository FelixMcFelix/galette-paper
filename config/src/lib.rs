pub mod experiment;
pub mod range;

use std::{io::Error as IoError, path::Path};

use ron::error::SpannedError as RonError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use toml::de::Error as TomlDeError;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Machine {
	pub name: String,
	pub hostname: Option<String>,
	pub arch: Option<String>,
	pub iface: Option<String>,
	pub dpdk_pcie_addr: Option<String>,
	pub um24c_addr: Option<String>,
	pub link_rate_mbps: Option<f64>,
	pub pktgen_home: Option<String>,
	pub mac_address: Option<String>,
	pub supports_power_measurement: Option<bool>,
	pub supports_dpdk_pmd: Option<bool>,
	pub driver_name: Option<String>,
}

impl Machine {
	pub fn hostname(&self) -> &str {
		if let Some(host) = &self.hostname {
			host
		} else {
			&self.name
		}
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub struct TomlList<T> {
	pub data: Vec<T>,
}

#[derive(Debug, Error)]
pub enum LoadFileError {
	#[error("failed to open and read file {path}")]
	OpenFile { path: String, source: IoError },
	#[error("failed to parse list of `{type_name}` from file at {path}")]
	Parse {
		path: String,
		type_name: &'static str,
		source: TomlDeError,
	},
	#[error("failed to parse experiment from file at {path}")]
	ParseExpt { path: String, source: RonError },
}

pub async fn load_file<T>(path: impl AsRef<Path>) -> Result<Vec<T>, LoadFileError>
where
	T: DeserializeOwned,
{
	let contents = tokio::fs::read_to_string(path.as_ref())
		.await
		.map_err(|source| LoadFileError::OpenFile {
			path: path.as_ref().to_string_lossy().into(),
			source,
		})?;

	let parsed: TomlList<T> =
		toml::de::from_str(&contents).map_err(|source| LoadFileError::Parse {
			path: path.as_ref().to_string_lossy().into(),
			type_name: std::any::type_name::<T>(),
			source,
		})?;

	Ok(parsed.data)
}

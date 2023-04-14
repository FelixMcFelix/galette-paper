use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{range::ExperimentRange, LoadFileError};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExperimentFile {
	pub cores: Option<ExperimentRange<u64>>,
	pub rate: Option<ExperimentRange<f64>>,
	pub upcall_percent: Option<ExperimentRange<f64>>,
	pub upcall_timeout: Option<ExperimentRange<u64>>,
	pub dut_machines: Option<Vec<String>>,
	pub dplanes: Option<Vec<NamedDplane>>,
	pub pkt_size: Option<ExperimentRange<u64>>,
	pub iterations: Option<u64>,

	pub fill_up_percent_fair: Option<bool>,

	pub prototype_path: Option<String>,
}

impl ExperimentFile {
	pub async fn load(path: impl AsRef<Path>) -> Result<Experiment, LoadFileError> {
		// start w default file
		// fill in missing Nones with values of prototype chain
		// create Experiment::default()
		// transplant all Somes into default.

		let contents = tokio::fs::read_to_string(path.as_ref())
			.await
			.map_err(|source| LoadFileError::OpenFile {
				path: path.as_ref().to_string_lossy().into(),
				source,
			})?;

		let mut parsed: ExperimentFile =
			ron::de::from_str(&contents).map_err(|source| LoadFileError::ParseExpt {
				path: path.as_ref().to_string_lossy().into(),
				source,
			})?;

		let mut path_root = PathBuf::from(path.as_ref());
		path_root.pop();

		while parsed.prototype_path.is_some() {
			let mut this_path = path_root.clone();
			this_path.push(parsed.prototype_path.as_ref().unwrap());

			let contents = tokio::fs::read_to_string(&this_path)
				.await
				.map_err(|source| LoadFileError::OpenFile {
					path: this_path.to_string_lossy().into(),
					source,
				})?;

			let new_parsed: ExperimentFile =
				ron::de::from_str(&contents).map_err(|source| LoadFileError::ParseExpt {
					path: this_path.to_string_lossy().into(),
					source,
				})?;

			if parsed.cores.is_none() {
				parsed.cores = new_parsed.cores;
			}
			if parsed.rate.is_none() {
				parsed.rate = new_parsed.rate;
			}
			if parsed.upcall_percent.is_none() {
				parsed.upcall_percent = new_parsed.upcall_percent;
			}
			if parsed.upcall_timeout.is_none() {
				parsed.upcall_timeout = new_parsed.upcall_timeout;
			}
			if parsed.dut_machines.is_none() {
				parsed.dut_machines = new_parsed.dut_machines;
			}
			if parsed.dplanes.is_none() {
				parsed.dplanes = new_parsed.dplanes;
			}
			if parsed.pkt_size.is_none() {
				parsed.pkt_size = new_parsed.pkt_size;
			}
			if parsed.iterations.is_none() {
				parsed.iterations = new_parsed.iterations;
			}

			if parsed.fill_up_percent_fair.is_none() {
				parsed.fill_up_percent_fair = new_parsed.fill_up_percent_fair;
			}

			parsed.prototype_path = new_parsed.prototype_path;
		}

		let mut out = Experiment::default();
		if let Some(v) = parsed.cores {
			out.cores = v;
		};
		if let Some(v) = parsed.rate {
			out.rate = v;
		};
		if let Some(v) = parsed.upcall_percent {
			out.upcall_percent = v;
		};
		if let Some(v) = parsed.upcall_timeout {
			out.upcall_timeout = v;
		};
		if let Some(v) = parsed.dut_machines {
			out.dut_machines = v;
		};
		if let Some(v) = parsed.dplanes {
			out.dplanes = v;
		};
		if let Some(v) = parsed.iterations {
			out.iterations = v;
		};
		if let Some(v) = parsed.pkt_size {
			out.pkt_size = v;
		};
		if let Some(v) = parsed.fill_up_percent_fair {
			out.fill_up_percent_fair = v;
		};

		Ok(out)
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NamedDplane {
	pub name: String,
	pub dplane: Dplane,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Dplane {
	/// Dplane name
	Trusded(String),
	TestpmdDpdk,
	TestpmdAfPacket,
}

#[derive(Clone, Debug)]
pub struct Experiment {
	pub cores: ExperimentRange<u64>,
	pub rate: ExperimentRange<f64>,
	pub upcall_percent: ExperimentRange<f64>,
	pub upcall_timeout: ExperimentRange<u64>,
	pub dut_machines: Vec<String>,
	pub dplanes: Vec<NamedDplane>,
	pub pkt_size: ExperimentRange<u64>,
	pub iterations: u64,
	pub fill_up_percent_fair: bool,
}

impl Experiment {
	// Already counted/named?
	// "l-%dB-%sM-%sC-%d%s.dat"
	// type (lat/power/tput), pktsz, rate, cores, iter, dplane as suffix
	// to include in folders:
	// upcall percent, poll i'val, dplane, time

	// pub fn expt_name_key_fields(&self) -> Vec<(usize, &str)> {
	// 	let mut out = vec![];

	// 	if self.cores.is_variable() {
	// 		out.push((0, "C"));
	// 	}
	// 	if self.rate.is_variable() {
	// 		out.push(1);
	// 	}
	// 	if self.cores.is_variable() {
	// 		out.push(0);
	// 	}

	// 	out
	// }
}

impl From<Experiment> for ExperimentFile {
	fn from(val: Experiment) -> Self {
		ExperimentFile {
			cores: Some(val.cores),
			rate: Some(val.rate),
			upcall_percent: Some(val.upcall_percent),
			upcall_timeout: Some(val.upcall_timeout),
			dut_machines: Some(val.dut_machines),
			dplanes: Some(val.dplanes),
			pkt_size: Some(val.pkt_size),
			iterations: Some(val.iterations),
			prototype_path: None,
			fill_up_percent_fair: Some(val.fill_up_percent_fair),
		}
	}
}

impl Default for Experiment {
	fn default() -> Self {
		Self {
			cores: ExperimentRange::Fixed(1),
			rate: ExperimentRange::Fixed(1.0),
			upcall_percent: ExperimentRange::Fixed(0.5),
			upcall_timeout: ExperimentRange::Fixed(1),
			dut_machines: vec!["rpi".into()],
			dplanes: vec![NamedDplane {
				name: "kxdp".into(),
				dplane: Dplane::Trusded("../examples/01-macswap-xdp".into()),
			}],
			pkt_size: ExperimentRange::List(vec![64, 128, 256, 512, 1024, 1280, 1518]),
			iterations: 10,
			fill_up_percent_fair: false,
		}
	}
}

// NEED: some way to compute dirname/filename that exposes what params are variable.

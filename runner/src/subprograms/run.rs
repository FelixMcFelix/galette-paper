use std::{collections::HashMap, ffi::OsStr, path::PathBuf, str::FromStr, time::Duration};

use backend::{message::*, sequencer::Sequencer};
use config::{
	experiment::{Experiment, ExperimentFile, NamedDplane, Dplane},
	Machine, range::ExperimentRange,
};
use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};

use crate::cli::Run;

async fn get_expts(params: &Run) -> anyhow::Result<Vec<PathBuf>> {
	let mut read_dir = tokio::fs::read_dir(&params.instances_dir).await?;

	let mut out = Vec::new();
	while let Some(dir) = read_dir.next_entry().await? {
		let as_path = dir.path();
		// as_path.set_extension("");
		// let local_path = as_path.strip_prefix(&params.instances_dir)?;
		// eprintln!("\t{:?}", local_path);
		if as_path.extension() == Some(OsStr::new("ron")) {
			out.push(as_path);
		}
	}

	Ok(out)
}

pub async fn run(params: Run) -> anyhow::Result<()> {
	if params.experiments.len() == 0 {
		eprintln!("No experiment specified, please specify \"!\" or one or more of:");
		let available = get_expts(&params).await?;
		for mut path in available {
			path.set_extension("");
			let local_path = path.strip_prefix(&params.instances_dir)?;
			eprintln!("\t{}", local_path.to_str().unwrap());
		}
		Ok(())
	} else {
		let paths = if params.experiments[0] == "!" {
			get_expts(&params).await?
		} else {
			params
				.experiments
				.iter()
				.map(|name| PathBuf::from(&params.instances_dir).join(name))
				.map(|mut path| {
					path.set_extension("ron");
					path
				})
				.collect()
		};

		// match ron::to_string(&ExperimentFile::from(Experiment::default())) {
		// 	Ok(a) => {
		// 		println!("TEST:\n{a:?}");
		// 	},
		// 	Err(e) => eprintln!("Type ser error {e}"),
		// }

		let mut expts = vec![];
		for path in paths {
			eprintln!("Loading expt {:?}", path);
			expts.push(ExperimentFile::load(path).await?)
		}

		let machines = config::load_file::<Machine>(&params.def_file).await?;

		let ts = chrono::offset::Local::now();
		let t_str = ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
		let t_str = t_str.replace(':', ".");

		for (expt, expt_name) in expts.iter_mut().zip(params.experiments.iter()) {
			let sanitised_name = expt_name.replace('/', "-").replace('\\', "-");

			for dut in expt.dut_machines.clone() {
				eprintln!("RUNNING: {sanitised_name}");
				run_one(&params, expt, &sanitised_name, &machines, &dut, &t_str).await?;
			}
		}

		Ok(())
	}
}

pub async fn run_one(
	params: &Run,
	experiment: &mut Experiment,
	experiment_name: &str,
	machines: &Vec<Machine>,
	dut_name: &str,
	time_str: &str,
) -> anyhow::Result<()> {
	let mut write_path = PathBuf::from(&params.key_dir);
	tokio::fs::create_dir_all(&write_path).await?;

	let mut t_certs = vec![];
	let mut needed_machines = (None, None);
	let mut runner_machine = None;
	for machine in machines {
		if machine.name == "runner" {
			runner_machine = Some(machine.clone());
		} else {
			write_path.push(format!("{}.cert.der", machine.hostname()));
			let cert = tokio::fs::read(&write_path).await?;
			write_path.pop();

			t_certs.push(cert);
		}

		if machine.name == dut_name {
			needed_machines.0 = Some(machine.clone());
		}

		if machine.name == params.pktgen_name {
			needed_machines.1 = Some(machine.clone());
		}
	}

	let (dut_machine, tester_machine) = match needed_machines {
		(Some(d), Some(t)) => (d, t),
		(None, _) => panic!(
			"machine {} did not correspond to entry of machine toml definition",
			dut_name
		),
		_ => panic!(
			"machine {} did not correspond to entry of machine toml definition",
			params.pktgen_name
		),
	};

	let runner_machine = runner_machine.expect("Need a machine 'runner' to be defined.");

	write_path.push("runner.key.der");
	let own_key = tokio::fs::read(&write_path).await?;
	write_path.pop();

	write_path.push("runner.cert.der");
	let own_cert = tokio::fs::read(&write_path).await?;
	write_path.pop();

	let stream = backend::stream::StreamCfg::new_cfg(own_cert, own_key, &t_certs[..])?;

	let cs = format!("wss://{}:{}", dut_machine.hostname(), backend::PORT);
	let mut dut_stream = stream.client_stream(cs.as_str()).await?;

	let cs = format!("wss://{}:{}", tester_machine.hostname(), backend::PORT);
	let mut tester_stream = stream.client_stream(cs.as_str()).await?;

	let mut local_stream = backend::local_device();

	// Actual doing of stuff.

	compile_and_distribute_binaries(
		&mut local_stream,
		&mut [
			(&mut dut_stream, &dut_machine),
			(&mut tester_stream, &tester_machine),
		],
	)
	.await?;

	// let dplanes = [
	// 	("../examples/01-macswap-xdp", "kxdp"),
	// 	("../examples/05-upcall", "uxdp"),
	// ];
	// let cores = [1]; //, 2, 3];
	// let pkt_size_len = 7;
	// let rates = [0.1, 0.5, 1.0, 10.0, 50.0, 100.0];

	for NamedDplane{name: dplane_short, dplane} in &experiment.dplanes {
		let compile_kill = if let Dplane::Trusded(t_name) = &dplane {
			Some(start_dplane_compiler(&mut local_stream, t_name).await)
		} else {
			None
		};
		// "01-macswap-xdp"

		// TODO: in mean time, configure dpdk on tester, dataplane [dpdk/pulley] on target

		bind_dpdk(&mut tester_stream, &tester_machine).await?;

		if let Dplane::Trusded(_) = &dplane {
			check_dplane_compiler_started().await;
		}

		// order == dplane -> prime measures -> start pktgen -> start measures -> await pktgen -> cleanup.

		for core_ct in &experiment.cores {
			if experiment.fill_up_percent_fair {
				// Balance between 1 XDP + core_ct Userland
				let n_cores = (1 + core_ct) as f64;
				experiment.upcall_percent = ExperimentRange::Fixed(1.0 - (1.0 / (n_cores)));
			}

			for upcall_percent in &experiment.upcall_percent {
				for upcall_timeout in &experiment.upcall_timeout {
					let dplane_seq = match dplane {
						Dplane::Trusded(_) => start_trusded_dplane(&mut dut_stream, &dut_machine, &runner_machine, core_ct, upcall_percent, upcall_timeout).await?,
						Dplane::TestpmdAfPacket => {
							// Reset to default driver if needed.
							if dut_machine.supports_dpdk_pmd.unwrap_or(false) {
								unbind_dpdk(&mut dut_stream, &dut_machine).await?;
							}

							// start dplane
							start_testpmd_dplane(&mut dut_stream, &dut_machine, true).await?
						},
						Dplane::TestpmdDpdk => {
							// bind dpdk
							bind_dpdk(&mut dut_stream, &dut_machine).await?;

							// start dplane
							start_testpmd_dplane(&mut dut_stream, &dut_machine, false).await?
						},
					};
					tokio::time::sleep(Duration::from_secs(5)).await;

					let _ = tester_stream.send(C2S::ResetResults).await;
					// WRITEOUT TIME.
					// "l-%dB-%sM-%sC-%d%s.dat"
					// type (lat/power/tput), pktsz, rate, cores, iter, dplane as suffix
					// to include in folders:
					// upcall percent, poll i'val, dplane, time
					let dplane_subfolder = match dplane {
						Dplane::Trusded(_) => format!("tru-{dplane_short}-{upcall_percent}p-{upcall_timeout}ms"),
						Dplane::TestpmdAfPacket => format!("testpmd-afp"),
						Dplane::TestpmdDpdk => format!("testpmd-dpdk"),
					};
					let out_dir = format!("results/{experiment_name}/{time_str}/{dut_name}/{dplane_subfolder}/");

					tokio::fs::create_dir_all(&out_dir).await?;

					for rate in &experiment.rate {
						for pkt_sz in &experiment.pkt_size {
							for iter in 0..experiment.iterations {
								tokio::time::sleep(Duration::from_secs(4)).await;
								println!(
									"RUNNING: {dplane_short}, {core_ct} cores, {rate}Mbps, sz_idx {pkt_sz}, iter {iter}"
								);
								// TODO: start measurement apparatus -- CPU (remote), toggle power based on dplane.

								let bt_seq = if dut_machine.supports_power_measurement.unwrap_or(false) {
									Some(tester_stream
										.send(C2S::MeasurePower(
											MeasurePower {
												mac_address: mac_address::MacAddress::from_str(
													&tester_machine.um24c_addr.as_ref().expect("Uh"),
												)
												.unwrap()
												.bytes(),
												poll_interval: Duration::from_millis(100),
												start_polling: false,
												max_measurements: 125.try_into().ok(),
											},
											None,
										))
										.await
										.unwrap())
								} else {
									None
								};

								let cpu_seq = dut_stream
									.send(C2S::MeasureCpu(
										MeasureCpu {
											poll_interval: Duration::from_millis(500),
											start_polling: false,
											max_measurements: 50.try_into().ok(),
										},
										None,
									))
									.await.unwrap();

								// start pktgen and power
								let pktgen_seq = start_pktgen(
									&mut tester_stream,
									params.back_to_back,
									&tester_machine,
									&dut_machine,
									core_ct,
									pkt_sz,
									rate,
									iter,
								)
								.await?;

								if let Some(seq) = bt_seq {
									tester_stream
										.send(C2S::ToggleMeasurement(seq))
										.await;	
								}

								dut_stream
									.send(C2S::ToggleMeasurement(cpu_seq))
									.await;	

								// await pktgen and power termination -- results done.
								// println!("Awaiting both pktgen, power...");
								tester_stream.recv_reply(pktgen_seq).await;
								// kill_seq(&mut tester_stream, pktgen_seq).await;

								let powers = if let Some(seq) = bt_seq {
									Some(tester_stream.recv_reply(seq).await)
								} else { None };
								// println!("{:?}", tester_stream.recv_reply(bt_seq).await);

								let cpus = dut_stream.recv_reply(cpu_seq).await;								

								if let Some(S2C::PowerData(p)) = powers {
									println!("got powers for idx {pkt_sz}");
									let mut wtr = csv::Writer::from_writer(vec![]);

									for power in p {
										wtr.serialize(power)?;
									}

									let opather = format!("{out_dir}p-{pkt_sz}B-{rate}M-{core_ct}C-{iter}.csv");
									tokio::fs::write(opather, wtr.into_inner()?).await?;
								}

								if let S2C::CpuData(c) = cpus {
									let opather = format!("{out_dir}c-{pkt_sz}B-{rate}M-{core_ct}C-{iter}.dat");
									tokio::fs::write(opather, c.join("\n")).await?;
								} else {
									eprintln!("Illegal CPU data: {:?}", cpus);
								}
							}
						}
					}

					// This downloads results excessively, I'm not too fussed.
					let dl_seq = tester_stream.send(C2S::Download(vec!["tmp/results".into()])).await.unwrap();
					if let S2C::Download(files) = tester_stream.recv_reply(dl_seq).await {
						for mut file in files {
							file.rename = Some(String::new());
							file.extract(&out_dir).await.unwrap();
						}
					} else {
						panic!("Sent illegal reply to download.");
					}

					kill_seq(&mut dut_stream, dplane_seq).await;

					// dplane cleanup
					match dplane {
						Dplane::TestpmdDpdk => unbind_dpdk(&mut dut_stream, &dut_machine).await?, // unbind?
						_ => {},
					}
				}
			}
			
		}

		if let Some(compile_kill) = compile_kill {
			kill_seq(&mut local_stream, compile_kill).await;
		}
	}

	Ok(())
}

async fn compile_and_distribute_binaries(
	local: &mut Sequencer,
	targets: &mut [(&mut Sequencer, &Machine)],
) -> anyhow::Result<()> {
	// local compile
	let cmd = C2S::Commands(vec![
		Command {
			command: vec![Invocation {
				prog: "cargo".into(),
				args: vecify(&["b", "--release"]),
			}],
			env: Default::default(),
			cwd: Some("../".into()),
		},
		Command {
			command: vec![Invocation {
				prog: "cargo".into(),
				args: vecify(&["b", "--release", "--target", "aarch64-unknown-linux-gnu"]),
			}],
			env: Default::default(),
			cwd: Some("../".into()),
		},
	]);

	let seq = local.send(cmd).await.unwrap();
	let resp = local.recv_reply(seq).await;
	println!("{resp:?}");

	// send & await
	let to_await = FuturesUnordered::new();
	for (stream, machine) in targets.iter_mut() {
		let mut path = PathBuf::from("../target");

		if let Some(extension) = &machine.arch {
			path.push(extension);
		}
		path.push("release");

		path.push("chainsmith");
		let mut chainsmith = SentFile::load(&path).await?;
		chainsmith.rename = Some("chainsmith".into());
		path.pop();

		path.push("pulley");
		let mut pulley = SentFile::load(&path).await?;
		pulley.rename = Some("pulley".into());
		path.pop();

		let cmd = C2S::Upload(vec![chainsmith, pulley]);
		to_await.push(stream.send(cmd))
	}

	to_await.collect::<Vec<_>>().await;

	for (stream, _) in targets {
		let cmd = C2S::Commands(vec![Command {
			command: vec![
				Invocation {
					prog: "chmod".into(),
					args: vecify(&["ugo+x", "tmp/chainsmith"]),
				},
				Invocation {
					prog: "chmod".into(),
					args: vecify(&["ugo+x", "tmp/pulley"]),
				},
			],
			env: Default::default(),
			cwd: None,
		}]);
		let chmod = stream.send(cmd).await.unwrap();
		stream.recv_reply(chmod).await;
	}

	Ok(())
}

async fn start_dplane_compiler(stream: &mut Sequencer, dplane: &str) -> Sequence {
	stream
		.send(C2S::Commands(vec![Command {
			command: vec![Invocation {
				prog: "../target/release/chainsmith".into(),
				args: vecify(&[dplane, "--conn-string", "0.0.0.0:8081"]),
			}],
			env: Default::default(),
			cwd: None,
		}]))
		.await
		.unwrap()
}

async fn check_dplane_compiler_started() {
	eprintln!("Checking port 8081...");
	while let Err(_e) = tokio::net::TcpStream::connect("localhost:8081").await {
		tokio::time::sleep(Duration::from_secs(5)).await;
	}
	eprintln!("Compiler live.");
}

async fn kill_seq(stream: &mut Sequencer, to_kill: Sequence) {
	let _ = stream.send(C2S::Terminate(to_kill)).await;
	let _ = stream.recv_reply(to_kill).await;
}

async fn bind_dpdk(stream: &mut Sequencer, machine: &Machine) -> anyhow::Result<()> {
	// enable unsafe vfio
	// bind dpdk
	// prime hugepages (dpdk-hugepages.py -p 1G --setup 2G)

	let pcie_str = machine.dpdk_pcie_addr.as_ref().ok_or(anyhow::anyhow!(
		"Device {} cannot be used as DPDK host",
		machine.name
	))?;

	let msg = C2S::Commands(vec![Command {
		command: vec![
			Invocation {
				prog: "bash".into(),
				args: vecify(&[
					"-c",
					"echo 1 > /sys/module/vfio/parameters/enable_unsafe_noiommu_mode",
				]),
			},
			Invocation {
				prog: "dpdk-devbind.py".into(),
				args: vecify(&["--bind=vfio-pci", pcie_str, "--force"]),
			},
			// Invocation {
			// 	prog: "dpdk-hugepages.py".into(),
			// 	args: vecify(&["-p", "1G", "--setup", "2G"]),
			// },
		],
		env: Default::default(),
		cwd: None,
	}]);

	let seq = stream.send(msg).await.unwrap();
	stream.recv_reply(seq).await;

	Ok(())
}

async fn unbind_dpdk(stream: &mut Sequencer, machine: &Machine) -> anyhow::Result<()> {
	// enable unsafe vfio
	// bind dpdk
	// prime hugepages (dpdk-hugepages.py -p 1G --setup 2G)

	let pcie_str = machine.dpdk_pcie_addr.as_ref().ok_or(anyhow::anyhow!(
		"Device {} cannot be used as DPDK host",
		machine.name
	))?;

	let msg = C2S::Commands(vec![Command {
		command: vec![
			// Invocation {
			// 	prog: "dpdk-devbind.py".into(),
			// 	args: vecify(&["-u", pcie_str]),
			// },
			Invocation {
				prog: "dpdk-devbind.py".into(),
				args: vecify(&["--bind=e1000e", pcie_str]),
			},
			// Invocation {
			// 	prog: "dpdk-hugepages.py".into(),
			// 	args: vecify(&["-c", "-u"]),
			// },
		],
		env: Default::default(),
		cwd: None,
	}]);

	let seq = stream.send(msg).await.unwrap();
	stream.recv_reply(seq).await;

	Ok(())
}

async fn start_pktgen(
	stream: &mut Sequencer,
	back_to_back: bool,
	pktgen_machine: &Machine,
	dut_machine: &Machine,
	core_ct: u64,
	pkt_sz: u64,
	target_mbps: f64,
	iter: u64,
) -> anyhow::Result<Sequence> {
	// sudo pktgen -n 6 -l 0-7 -- -m "2.0"

	// if expt in script add "-f x.lua"

	let max_mbps = if back_to_back {
		dut_machine.link_rate_mbps.ok_or(anyhow::anyhow!(
			"Machine {} does not have a known link rate.",
			dut_machine.name
		))?
	} else {
		1000.0
	};
	let rate_frac = target_mbps / max_mbps;

	// Weirdly, Pktgem rates cap out at 10 for 1Gbps, so rescale.
	let rate_frac = 10.0 * rate_frac;

	const SCRIPT: &str = "pktgen-measure.lua";

	let msg = C2S::Upload(vec![SentFile::load(SCRIPT).await?]);
	let _ = stream.send(msg).await;

	let mut env = HashMap::new();
	env.insert("TEST_STRESS_ITER".into(), format!("{iter}"));
	env.insert("TEST_STRESS_RATE".into(), format!("{rate_frac}"));
	env.insert("TEST_STRESS_RATE_DISPLAY".into(), format!("{target_mbps}"));
	env.insert("TEST_STRESS_SZ".into(), format!("{pkt_sz}"));
	env.insert("TEST_STRESS_CORE_CT".into(), format!("{core_ct}"));
	env.insert("TEST_STRESS_SUFFIX".into(), format!(""));
	if let Some(mac) = &dut_machine.mac_address {
		env.insert("TEST_STRESS_DST_MAC".into(), mac.clone());
	}
	env.insert(
		"PKTGEN_HOME".into(),
		pktgen_machine
			.pktgen_home
			.as_ref()
			.ok_or(anyhow::anyhow!(
				"FATAL: Need PktGen install directory (`pktgen_home`) on {}",
				pktgen_machine.name
			))?
			.into(),
	);

	let msg = C2S::Commands(vec![Command {
		command: vec![Invocation {
			prog: "pktgen".into(),
			args: vecify(&[
				"-n",
				"2", // divined via http://mails.dpdk.org/archives/dev/2013-June/000226.html
				"-l",
				"0,2,4", //
				"--",    //
				"-m",
				"[2:4].0", //
				"-f",
				"tmp/pktgen-measure.lua",
			]),
		}],
		env,
		cwd: None,
	}]);

	Ok(stream.send(msg).await.unwrap())
}

async fn start_trusded_dplane(
	stream: &mut Sequencer,
	machine: &Machine,
	compiler_machine: &Machine,
	core_ct: u64,
	upcall_percent: f64,
	upcall_timeout: u64,
) -> anyhow::Result<Sequence> {
	let iface_name = machine.iface.as_ref().ok_or(anyhow::anyhow!(
		"Device {} cannot be used as TruSDEd host",
		machine.name
	))?;

	let mut args = vecify(&["-i", iface_name]);
	args.push("--xdp-cores".into());
	args.push(core_ct.to_string());

	args.push("--share-umem".into());

	args.push("--loadbalance-chance".into());
	args.push(upcall_percent.to_string());

	args.push("--upcall-poll-timeout".into());
	args.push(upcall_timeout.to_string());

	args.push(format!("ws://{}:8080", compiler_machine.hostname()));

	// TODO: add --xdp-cores, --share-umem as needed,

	let msg = C2S::Commands(vec![Command {
		command: vec![
			Invocation {
				prog: "ip".into(),
				args: vecify(&["link", "set", iface_name, "promisc", "on"]),
			},
			Invocation {
				prog: "tmp/pulley".into(),
				args,
			},
		],
		env: Default::default(),
		cwd: None,
	}]);

	Ok(stream.send(msg).await.unwrap())
}

async fn start_testpmd_dplane(
	stream: &mut Sequencer,
	machine: &Machine,
	use_af_packet: bool,
) -> anyhow::Result<Sequence> {
	let iface_name = machine.iface.as_ref().ok_or(anyhow::anyhow!(
		"Device {} cannot be used as TruSDEd host",
		machine.name
	))?;

	let mut cmds = vec![];

	if use_af_packet {
		cmds.push(Invocation {
			prog: "ip".into(),
			args: vecify(&["link", "set", iface_name, "promisc", "on"]),
		});
	}

	let mut args = vecify(&["-l", "0,2"]);

	if !machine.supports_dpdk_pmd.unwrap_or(false) {
		// below is needed for pi.
		// --no-huge -m 640
		args.push("--no-huge".into());
		args.push("-m".into());
		args.push("640".into());
	}

	if use_af_packet {
		args.push(format!("--vdev=eth_af_packet0,iface={iface_name},blocksz=4096,framesz=2048,framecnt=2048,qdisc_bypass=1"));
	}

	args.push("--".into());
	args.push("--forward-mode=macswap".into());

	cmds.push(Invocation {
		prog: "dpdk-testpmd".into(),
		args,
	});

	let msg = C2S::Commands(vec![Command {
		command: cmds,
		env: Default::default(),
		cwd: None,
	}]);

	Ok(stream.send(msg).await.unwrap())
}

fn vecify(data: &[&str]) -> Vec<String> {
	data.iter().map(|v| String::from(*v)).collect()
}

pub mod message;
pub mod sequencer;
pub mod server;
pub mod stream;

#[cfg(feature = "bt")]
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};

use flume::{Receiver, Sender};
use message::*;
use sequencer::Sequencer;
use server::*;
#[cfg(feature = "bt")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command as TokCommand;
#[cfg(feature = "bt")]
use tokio_serial::SerialPortBuilderExt;

pub const PORT: u16 = 9832;

pub async fn serve(
	own_cert: Vec<u8>,
	own_key: Vec<u8>,
	trusted_clients: &[Vec<u8>],
) -> Result<Server, ServerError> {
	Server::new(
		&format!("0.0.0.0:{PORT}"),
		own_cert,
		own_key,
		trusted_clients,
	)
	.await
}

pub fn c2s_doer() -> (Sender<Message<C2S>>, Receiver<Message<S2C>>) {
	let (c2s_tx, c2s_rx) = flume::unbounded();
	let (s2c_tx, s2c_rx) = flume::unbounded();

	tokio::spawn(async move { command_runner(c2s_rx, s2c_tx).await });

	(c2s_tx, s2c_rx)
}

pub fn local_device() -> Sequencer {
	Sequencer::new(c2s_doer())
}

async fn command_runner(rx: Receiver<Message<C2S>>, tx: Sender<Message<S2C>>) {
	let mut kill_chans = HashMap::new();
	let mut power_chans: HashMap<Sequence, Sender<MeasureControl<MeasurePower>>> = HashMap::new();
	let mut cpu_chans: HashMap<Sequence, Sender<MeasureControl<MeasureCpu>>> = HashMap::new();

	while let Ok(msg) = rx.recv_async().await {
		match msg.cmd {
			C2S::Ping => {
				let reply = Message {
					seq: msg.seq,
					cmd: S2C::Pong,
				};

				let _ = tx.send_async(reply).await;
			},
			C2S::Commands(cmd) => {
				let remote_tx = tx.clone();
				let (kill_tx, kill_rx) = flume::bounded(1);

				kill_chans.insert(msg.seq, kill_tx);

				tokio::spawn(
					async move { do_command_block(cmd, remote_tx, kill_rx, msg.seq).await },
				);
			},
			C2S::Terminate(seq) =>
				if let Some(killer) = kill_chans.remove(&seq) {
					let _ = killer.send(());
				},
			C2S::Download(reqs) => {
				let mut out = Vec::with_capacity(reqs.len());
				for p in reqs {
					out.push(
						SentFile::load(&p)
							.await
							.expect(&format!("Failed to download path {p}")),
					);
				}
				let _ = tx.send(Message {
					seq: msg.seq,
					cmd: S2C::Download(out),
				});
			},
			C2S::Upload(files) =>
				for file in files {
					if let Err(e) = file.extract("tmp").await {
						eprintln!("Failed to create file: {:?}", file.filename);
					}
				},
			C2S::MeasurePower(setup, maybe_sequence) =>
				if let Some(sequence) = maybe_sequence {
					if let Some(ctl) = power_chans.get(&sequence) {
						let _ = ctl.send_async(MeasureControl::NewState(setup)).await;
					} else {
						let reply = Message {
							seq: msg.seq,
							cmd: S2C::Error(format!(
								"Could not update MeasurePower({}): does not exist",
								sequence.0
							)),
						};

						let _ = tx.send_async(reply).await;
					}
				} else {
					let remote_tx = tx.clone();
					let (kill_tx, kill_rx) = flume::bounded(1);
					let (ctl_tx, ctl_rx) = flume::unbounded();

					kill_chans.insert(msg.seq, kill_tx);
					power_chans.insert(msg.seq, ctl_tx);

					tokio::spawn(async move {
						measure_power(setup, remote_tx, kill_rx, ctl_rx, msg.seq).await
					});
				},

			C2S::MeasureCpu(setup, maybe_sequence) =>
				if let Some(sequence) = maybe_sequence {
					if let Some(ctl) = cpu_chans.get(&sequence) {
						let _ = ctl.send_async(MeasureControl::NewState(setup)).await;
					} else {
						let reply = Message {
							seq: msg.seq,
							cmd: S2C::Error(format!(
								"Could not update MeasurePower({}): does not exist",
								sequence.0
							)),
						};

						let _ = tx.send_async(reply).await;
					}
				} else {
					let remote_tx = tx.clone();
					let (kill_tx, kill_rx) = flume::bounded(1);
					let (ctl_tx, ctl_rx) = flume::unbounded();

					kill_chans.insert(msg.seq, kill_tx);
					cpu_chans.insert(msg.seq, ctl_tx);

					tokio::spawn(async move {
						measure_cpu(setup, remote_tx, kill_rx, ctl_rx, msg.seq).await
					});
				},
			C2S::ToggleMeasurement(seq) =>
				{
					if let Some(ctl) = power_chans.get(&seq) {
						let _ = ctl.send_async(MeasureControl::Toggle).await;
					}
					if let Some(ctl) = cpu_chans.get(&seq) {
						let _ = ctl.send_async(MeasureControl::Toggle).await;
					}
				},
			C2S::ResetResults => {
				let _ = tokio::fs::remove_dir_all("tmp/results").await;
				let _ = tokio::fs::create_dir_all("tmp/results").await;
			},
		}
	}
}

async fn do_command_block(
	cmds: Vec<Command>,
	resp: Sender<Message<S2C>>,
	kill: Receiver<()>,
	seq: Sequence,
) {
	let mut exits = vec![];

	'exit: for cmd in cmds {
		let mut out = None;
		for sub_cmd in cmd.command {
			let mut invocation = TokCommand::new(&sub_cmd.prog);

			invocation.args(&sub_cmd.args);
			invocation.envs(&cmd.env);
			invocation.kill_on_drop(true);
			// invocation.stdin(Stdio::piped());

			if let Some(cwd) = &cmd.cwd {
				invocation.current_dir(cwd);
			}

			// let mut child = invocation.spawn().unwrap();

			eprintln!("DOING COMMAND: {:#?}", invocation);

			// TODO: possibly sleep-poll completion, and check for kill each time?
			let this_out = tokio::select! {
				res = invocation.output() => { res },
				_ = kill.recv_async() => {
					// child.kill().await;
					break 'exit;
				}
			};

			match this_out {
				Ok(o) => {
					println!("{:?}", std::str::from_utf8(&o.stderr));
					out = Some((o.status.code().unwrap_or_default(), o.stdout, o.stderr));
				},
				Err(e) => {
					println!("{e:?}, {e}");
					exits.push((-1, vec![], format!("{e:?}").as_bytes().to_vec()));

					break;
				},
			}
		}

		if let Some(exit) = out {
			exits.push(exit);
		}
	}

	let msg = Message {
		seq,
		cmd: S2C::CommandDone(exits),
	};

	let _ = resp.send_async(msg).await;
}

enum MeasureControl<T> {
	Toggle,
	NewState(T),
}

#[cfg(feature = "bt")]
async fn measure_power(
	cfg: MeasurePower,
	resp: Sender<Message<S2C>>,
	kill: Receiver<()>,
	ctl: Receiver<MeasureControl<MeasurePower>>,
	seq: Sequence,
) {
	let mut msg = Message {
		seq,
		cmd: S2C::Pong, // guaranteed to be replaced before send.
	};

	match measure_power_inner(cfg, kill, ctl).await {
		Ok(results) => msg.cmd = S2C::PowerData(results),
		Err(e) => {
			msg.cmd = S2C::Error(format!("Power measurement fatal error: {}", e));
		},
	}

	let _ = resp.send_async(msg).await;
}

#[cfg(feature = "bt")]
async fn measure_power_inner(
	mut cfg: MeasurePower,
	kill: Receiver<()>,
	ctl: Receiver<MeasureControl<MeasurePower>>,
) -> anyhow::Result<Vec<PowerSample>> {
	// rfcomm binding according to:
	// https://gist.github.com/0/c73e2557d875446b9603

	use std::time::Instant;
	let mac_str = cfg.mac_address.map(|v| format!("{v:02x?}")).join(":");

	TokCommand::new("rfcomm")
		.args(&["bind", "0", &mac_str])
		.output()
		.await?;

	const CMD_WAIT: Duration = Duration::from_millis(200);

	// do stuff
	let mut port = tokio_serial::new("/dev/rfcomm0", 9600).open_native_async()?;

	// Byte indices and commands known via:
	// https://sigrok.org/wiki/RDTech_UM_series
	port.write_all(&[0xf4]).await?; // reset stats.
	tokio::time::sleep(CMD_WAIT).await;

	let mut out = vec![];
	let mut next_measurement = Instant::now();
	loop {
		let mut resp_buf = [0u8; 130];

		if Some(out.len() as u64) >= cfg.max_measurements.map(|v| v.get())
			|| kill.try_recv().is_ok()
		{
			break;
		}

		match ctl.try_recv() {
			Ok(MeasureControl::Toggle) => cfg.start_polling = !cfg.start_polling,
			Ok(MeasureControl::NewState(n)) => {
				cfg.poll_interval = n.poll_interval;
				cfg.start_polling = n.start_polling;
				cfg.max_measurements = n.max_measurements;
			},
			_ => {},
		}

		if cfg.start_polling {
			port.write_all(&[0xf0]).await?;
			if tokio::time::timeout(Duration::from_millis(2000), port.read_exact(&mut resp_buf))
				.await
				.is_err()
			{
				next_measurement = Instant::now();
				println!("Req lost, retrying.");
				continue;
			}

			out.push(PowerSample {
				total_m_watt_hour: gu32(&resp_buf[16..]),
				total_m_amp_hour: gu32(&resp_buf[20..]),
				m_watts: gu32(&resp_buf[6..]),
				m_volts: gu16(&resp_buf[2..]),
				m_amps: gu16(&resp_buf[4..]),
			});

			println!("Got {}/{:?} measurements", out.len(), cfg.max_measurements);

			next_measurement += cfg.poll_interval;
			tokio::time::sleep_until(next_measurement.into()).await;
		} else {
			tokio::time::sleep(CMD_WAIT).await;
			next_measurement = Instant::now();
		}
	}

	//

	TokCommand::new("rfcomm")
		.args(&["release", "0"])
		.output()
		.await?;

	Ok(out)
}

#[cfg(not(feature = "bt"))]
async fn measure_power(
	_: MeasurePower,
	resp: Sender<Message<S2C>>,
	_: Receiver<()>,
	_: Receiver<MeasureControl>,
	seq: Sequence,
) {
	let msg = Message {
		seq,
		cmd: S2C::Error("Target machine does not support bluetooth".into()),
	};
	let _ = resp.send_async(msg).await;
}

#[cfg(unix)]
async fn measure_cpu(
	cfg: MeasureCpu,
	resp: Sender<Message<S2C>>,
	kill: Receiver<()>,
	ctl: Receiver<MeasureControl<MeasureCpu>>,
	seq: Sequence,
) {
	let mut msg = Message {
		seq,
		cmd: S2C::Pong, // guaranteed to be replaced before send.
	};

	match measure_cpu_inner(cfg, kill, ctl).await {
		Ok(results) => msg.cmd = S2C::CpuData(results),
		Err(e) => {
			msg.cmd = S2C::Error(format!("Cpu measurement fatal error: {}", e));
		},
	}

	let _ = resp.send_async(msg).await;
}

#[cfg(unix)]
async fn measure_cpu_inner(
	mut cfg: MeasureCpu,
	kill: Receiver<()>,
	ctl: Receiver<MeasureControl<MeasureCpu>>,
) -> anyhow::Result<Vec<String>> {
	// rfcomm binding according to:
	// https://gist.github.com/0/c73e2557d875446b9603

	use std::time::Instant;

	let mut out = vec![];
	let mut next_measurement = Instant::now();
	loop {
		let mut resp_buf = [0u8; 130];

		if Some(out.len() as u64) >= cfg.max_measurements.map(|v| v.get())
			|| kill.try_recv().is_ok()
		{
			break;
		}

		match ctl.try_recv() {
			Ok(MeasureControl::Toggle) => cfg.start_polling = !cfg.start_polling,
			Ok(MeasureControl::NewState(n)) => cfg = n,
			_ => {},
		}

		if cfg.start_polling {
			out.push(tokio::fs::read_to_string("/proc/stat").await?);

			println!("Got {}/{:?} CPU measurements", out.len(), cfg.max_measurements);

			next_measurement += cfg.poll_interval;
			tokio::time::sleep_until(next_measurement.into()).await;
		} else {
			tokio::time::sleep(Duration::from_millis(200)).await;
			next_measurement = Instant::now();
		}
	}

	Ok(out)
}

#[cfg(not(unix))]
async fn measure_cpu(
	_: MeasureCpu,
	resp: Sender<Message<S2C>>,
	_: Receiver<()>,
	_: Receiver<MeasureControl<MeasureCpu>>,
	seq: Sequence,
) {
	let msg = Message {
		seq,
		cmd: S2C::Error("Target machine does not support bluetooth".into()),
	};
	let _ = resp.send_async(msg).await;	
}


fn gu32(buf: &[u8]) -> u32 {
	u32::from_be_bytes(buf[..std::mem::size_of::<u32>()].try_into().unwrap())
}

fn gu16(buf: &[u8]) -> u16 {
	u16::from_be_bytes(buf[..std::mem::size_of::<u16>()].try_into().unwrap())
}

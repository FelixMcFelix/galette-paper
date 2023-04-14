use std::net::SocketAddr;

use backend::stream::*;
use cli::Cli;

pub mod cli;

pub async fn run(params: Cli) -> anyhow::Result<()> {
	let own_cert =
		tokio::fs::read(format!("{}/{}.cert.der", params.key_dir, params.my_name)).await?;
	let own_key = tokio::fs::read(format!("{}/{}.key.der", params.key_dir, params.my_name)).await?;
	let remote_cert = tokio::fs::read(format!(
		"{}/{}.cert.der",
		params.key_dir, params.client_name
	))
	.await?;

	let server = backend::serve(own_cert, own_key, &[remote_cert]).await?;

	loop {
		let maybe_stream = server.get_stream().await;
		match maybe_stream {
			Ok(conn) => {
				tokio::spawn(async move { server_loop(conn.0, conn.1).await });
			},
			Err(e) => eprintln!("{e:?}"),
		}
	}

	#[allow(unreachable_code)]
	Ok(())
}

async fn server_loop(mut stream: DeviceStream, addr: SocketAddr) {
	println!("Connection arrived from {addr}.");
	let (doer_tx, doer_rx) = backend::c2s_doer();

	loop {
		let exit = tokio::select! {
			msg = stream.recv() => {
				match msg {
					Err(e) if !e.is_nonfatal() => {
						eprintln!("Addr closed: {e:?}");
						true
					}
					Ok(msg) => doer_tx.send_async(msg).await.is_err(),
					_ => false,
				}
			},
			inner_msg = doer_rx.recv_async() => {
				match inner_msg {
					Ok(reply) => {
						let res = stream.send(&reply).await;

						res.is_err()
					}
					_ => true,
				}
			}
		};

		if exit {
			break;
		}
	}
}

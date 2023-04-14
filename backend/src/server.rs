use std::{io::Error as IoError, net::SocketAddr, sync::Arc};

use thiserror::Error;
use tokio::net::TcpListener;
use tokio_rustls::{
	rustls::{
		server::AllowAnyAuthenticatedClient,
		Certificate,
		Error as RustlsError,
		PrivateKey,
		ServerConfig,
	},
	TlsAcceptor,
	TlsStream,
};
use tokio_tungstenite::tungstenite::{protocol::WebSocketConfig, Error as WsError};

use crate::stream::*;

#[derive(Debug, Error)]
pub enum ServerError {
	#[error("failed to bind address {conn_string} to host server")]
	Bind {
		conn_string: String,
		source: IoError,
	},
	#[error("failed to build client config from supplied certificates")]
	Config(#[from] RustlsError),
}

#[derive(Debug, Error)]
pub enum StreamCreateError {
	#[error("failed")]
	Accept(#[from] IoError),
	#[error("failed to connect to {server} during TLS handshake")]
	TlsHandshake { server: String, source: IoError },
	#[error("failed to connect to {server} during websocket handshake")]
	WsHandshake { server: String, source: WsError },
}

pub struct Server {
	socket: TcpListener,
	config: Arc<ServerConfig>,
}

impl Server {
	pub(crate) async fn new(
		conn_string: &str,
		own_cert: Vec<u8>,
		own_key: Vec<u8>,
		trusted_clients: &[Vec<u8>],
	) -> Result<Self, ServerError> {
		let socket = TcpListener::bind(conn_string)
			.await
			.map_err(|source| ServerError::Bind {
				conn_string: conn_string.into(),
				source,
			})?;

		let mut trust = tokio_rustls::rustls::RootCertStore::empty();
		trust.add_parsable_certificates(trusted_clients);

		let config = ServerConfig::builder()
			.with_safe_defaults()
			.with_client_cert_verifier(AllowAnyAuthenticatedClient::new(trust))
			.with_single_cert(vec![Certificate(own_cert)], PrivateKey(own_key))?
			.into();

		Ok(Self { socket, config })
	}

	pub async fn get_stream(&self) -> Result<(DeviceStream, SocketAddr), StreamCreateError> {
		let (stream, addr) = self.socket.accept().await?;

		let tls = TlsAcceptor::from(Arc::clone(&self.config));
		let stream =
			tls.accept(stream)
				.await
				.map_err(|source| StreamCreateError::TlsHandshake {
					server: addr.to_string(),
					source,
				})?;

		let mut ws_config = WebSocketConfig::default();
		ws_config.max_frame_size = Some(usize::MAX);
		ws_config.max_message_size = Some(usize::MAX);

		let ws_stream =
			tokio_tungstenite::accept_async_with_config(TlsStream::from(stream), Some(ws_config))
				.await
				.map_err(|source| StreamCreateError::WsHandshake {
					server: addr.to_string(),
					source,
				})?;

		eprintln!("WS made with config: {:?}", ws_stream.get_config());

		Ok((Stream::new(ws_stream), addr))
	}
}

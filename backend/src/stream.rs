use std::{marker::PhantomData, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use postcard::Error as DeserError;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_rustls::{
	rustls::{Certificate, ClientConfig, Error as RustlsError, PrivateKey, RootCertStore},
	TlsStream,
};
use tokio_tungstenite::{
	tungstenite::{
		protocol::{Message as WsMsg, WebSocketConfig},
		Error as WsError,
	},
	Connector,
	MaybeTlsStream,
	WebSocketStream,
};

use crate::{message::*, sequencer::Sequencer};

pub struct StreamCfg(Arc<ClientConfig>);
pub type RunnerStream = Stream<Message<C2S>, Message<S2C>>;
pub type DeviceStream = Stream<Message<S2C>, Message<C2S>>;

type CStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type SStream = WebSocketStream<TlsStream<TcpStream>>;

pub(crate) enum InnerStream {
	Client(CStream),
	Server(SStream),
}

impl From<CStream> for InnerStream {
	fn from(val: CStream) -> Self {
		Self::Client(val)
	}
}

impl From<SStream> for InnerStream {
	fn from(val: SStream) -> Self {
		Self::Server(val)
	}
}

pub struct Stream<I, O> {
	stream: InnerStream,
	in_ty: PhantomData<I>,
	out_ty: PhantomData<O>,
}

#[derive(Debug, Error)]
pub enum StreamCfgError {
	#[error("failed to build client config from supplied certificates")]
	Build(#[from] RustlsError),
}

#[derive(Debug, Error)]
pub enum StreamCreateError {
	#[error("failed to connect to {server}")]
	Connect { server: String, source: WsError },
	#[error("could not establish TLS connection to {server}")]
	NonTls { server: String },
}

impl StreamCfg {
	pub fn new_cfg(
		own_cert: Vec<u8>,
		own_key: Vec<u8>,
		trusted_servers: &[Vec<u8>],
	) -> Result<Self, StreamCfgError> {
		let mut trust = RootCertStore::empty();
		trust.add_parsable_certificates(trusted_servers);

		let cfg = ClientConfig::builder()
			.with_safe_defaults()
			.with_root_certificates(trust)
			.with_single_cert(vec![Certificate(own_cert)], PrivateKey(own_key))?;

		Ok(Self(Arc::new(cfg)))
	}

	pub async fn client_stream(&self, conn_string: &str) -> Result<Sequencer, StreamCreateError> {
		let connector = Connector::Rustls(Arc::clone(&self.0));

		let mut ws_config = WebSocketConfig::default();
		ws_config.max_frame_size = Some(usize::MAX);
		ws_config.max_message_size = Some(usize::MAX);

		let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
			conn_string,
			Some(ws_config),
			Some(connector),
		)
		.await
		.map_err(|source| StreamCreateError::Connect {
			server: conn_string.into(),
			source,
		})?;

		if let MaybeTlsStream::Plain(_) = &stream.get_ref() {
			return Err(StreamCreateError::NonTls {
				server: conn_string.into(),
			});
		}

		Ok(Sequencer::new(Stream::new(stream)))
	}
}

#[derive(Debug, Error)]
pub enum SendError {
	#[error("unable to send binary message to other client")]
	DoSend(#[from] WsError),
}

#[derive(Debug, Error)]
pub enum RecvError {
	#[error("websocket stream is closed")]
	Closed,
	#[error("encountered a fatal error while receiving a message")]
	UnexpectedError(#[from] WsError),
	#[error("protocol violation, partner sent WS text message")]
	NonBinaryWs,
	#[error("protocol violation, partner sent message of unexpected type")]
	IllegalMessage(#[from] DeserError),
	#[error("received ping/pong -- should continue without closure")]
	AuxMessage,
}

impl RecvError {
	pub fn is_nonfatal(&self) -> bool {
		matches!(self, Self::AuxMessage)
	}
}

impl<I, O> Stream<I, O>
where
	I: Serialize,
	O: DeserializeOwned,
{
	pub(crate) fn new(inner: impl Into<InnerStream>) -> Self {
		Self {
			stream: inner.into(),
			in_ty: PhantomData::default(),
			out_ty: PhantomData::default(),
		}
	}

	pub async fn recv(&mut self) -> Result<O, RecvError> {
		let msg = match &mut self.stream {
			InnerStream::Server(s) => s.next().await,
			InnerStream::Client(c) => c.next().await,
		};

		let msg = if let Some(msg) = msg {
			msg
		} else {
			return Err(RecvError::Closed);
		};

		match msg? {
			WsMsg::Binary(b) => Ok(postcard::from_bytes(&b)?),
			WsMsg::Ping(_) | WsMsg::Pong(_) => Err(RecvError::AuxMessage),
			WsMsg::Close(_) => Err(RecvError::Closed),
			_ => Err(RecvError::NonBinaryWs),
		}
	}

	pub async fn send(&mut self, msg: &I) -> Result<(), SendError> {
		let msg = postcard::to_stdvec(msg)
			.map(WsMsg::Binary)
			.expect("FATAL: message could not be encoded.");

		match &mut self.stream {
			InnerStream::Server(s) => s.send(msg).await?,
			InnerStream::Client(c) => c.send(msg).await?,
		}

		Ok(())
	}
}

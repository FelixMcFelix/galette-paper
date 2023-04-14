use std::collections::HashMap;

use flume::{Receiver, Sender};

use crate::{message::*, stream::RunnerStream};

pub struct Sequencer {
	pub seq: Sequence,
	pub backend: StreamBackend,
	pub replies: HashMap<Sequence, S2C>,
}

impl Sequencer {
	pub fn new(stream: impl Into<StreamBackend>) -> Self {
		Self {
			seq: Sequence(0),
			backend: stream.into(),
			replies: HashMap::new(),
		}
	}

	pub async fn send(&mut self, cmd: C2S) -> Option<Sequence> {
		let seq = self.seq;
		let reply_expected = cmd.has_reply();
		let msg = Message { seq, cmd };
		self.seq.0 += 1;

		self.backend.send(msg).await;

		reply_expected.then_some(seq)
	}

	pub async fn recv_reply(&mut self, seq: Sequence) -> S2C {
		if let Some(val) = self.replies.remove(&seq) {
			return val;
		}

		loop {
			let msg = self.backend.recv().await;

			if msg.seq == seq {
				return msg.cmd;
			} else {
				self.replies.insert(msg.seq, msg.cmd);
			}
		}
	}
}

pub enum StreamBackend {
	Ws(RunnerStream),
	Local(Sender<Message<C2S>>, Receiver<Message<S2C>>),
}

impl StreamBackend {
	pub async fn send(&mut self, msg: Message<C2S>) {
		match self {
			Self::Ws(s) => s.send(&msg).await.unwrap(),
			Self::Local(tx, _) => tx.send_async(msg).await.unwrap(),
		}
	}

	pub async fn recv(&mut self) -> Message<S2C> {
		match self {
			Self::Ws(s) => s.recv().await.unwrap(),
			Self::Local(_, rx) => rx.recv_async().await.unwrap(),
		}
	}
}

impl From<RunnerStream> for StreamBackend {
	fn from(v: RunnerStream) -> Self {
		Self::Ws(v)
	}
}

impl From<(Sender<Message<C2S>>, Receiver<Message<S2C>>)> for StreamBackend {
	fn from(v: (Sender<Message<C2S>>, Receiver<Message<S2C>>)) -> Self {
		Self::Local(v.0, v.1)
	}
}

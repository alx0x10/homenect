use std::{
	collections::HashSet,
	sync::{
		atomic::{AtomicU64, Ordering},
		Arc,
	},
};

use iroh::{
	endpoint::Connection,
	protocol::{AcceptError, ProtocolHandler},
	Endpoint,
	NodeId,
};
use iroh_blobs::{store::fs::FsStore, ticket::BlobTicket};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tracing::{debug, error, info, instrument};

/// ALPN for our tiny control plane.
pub const CONTROL_ALPN: &str = "/homenect/control/1";

/// A hard upper bound for control messages (JSON).
const CONTROL_MAX_BYTES: usize = 4 * 1024 * 1024; // 4 MiB

/// Client → Server: ask the Pi to pull these tickets.
#[derive(Debug, Deserialize)]
pub struct BeginBackup {
	pub device_tag: String,
	pub tickets: Vec<String>,
}

/// Server → Client: completion
#[derive(Debug, Serialize)]
pub struct CompletionAck {
	pub job_id: u64,
	pub ok: bool,
	pub downloaded: usize,
	pub failed: usize,
	pub error: Option<String>,
}

/// Control-level errors (no `anyhow`).
#[derive(Debug, Snafu)]
pub enum ControlError {
	#[snafu(display("unauthorized peer"))]
	Unauthorized,

	// read_to_end returns iroh_quinn::ReadToEndError; keep it boxed for stability.
	#[snafu(display("read failed: {source}"))]
	Read {
		source: Box<dyn std::error::Error + Send + Sync>,
	},

	#[snafu(display("parse failed: {source}"))]
	Parse {
		source: serde_json::Error,
	},

	#[snafu(display("ticket parse failed: {ticket}: {source}"))]
	Ticket {
		ticket: String,
		source: Box<dyn std::error::Error + Send + Sync>,
	},

	#[snafu(display("download failed: {source}"))]
	Download {
		source: Box<dyn std::error::Error + Send + Sync>,
	},

	// write_all/finish return iroh_quinn errors; convert via Into<io::Error>.
	#[snafu(display("reply failed: {source}"))]
	Reply {
		source: std::io::Error,
	},
}

/// Control handler: auth by NodeId, read JSON, pull blobs via FsStore downloader.
#[derive(Debug)]
pub struct ControlHandler {
	allow_node_ids: HashSet<NodeId>,
	job_seq: Arc<AtomicU64>,
	endpoint: Arc<Endpoint>,
	store: Arc<FsStore>,
}

impl ControlHandler {
	pub fn new(
		allow_node_ids: HashSet<NodeId>,
		endpoint: Arc<Endpoint>,
		store: Arc<FsStore>,
	) -> Arc<Self> {
		Arc::new(Self {
			allow_node_ids,
			job_seq: Arc::new(AtomicU64::new(1)),
			endpoint,
			store,
		})
	}

	#[inline]
	pub fn alpn() -> &'static [u8] {
		CONTROL_ALPN.as_bytes()
	}
}

impl ProtocolHandler for ControlHandler {
	#[instrument(level = "debug", skip(self, conn))]
	fn accept(
		&self,
		conn: Connection,
	) -> impl core::future::Future<Output = Result<(), AcceptError>> + Send {
		// Clone the pieces needed to build a 'static future.
		let allow_node_ids = self.allow_node_ids.clone();
		let endpoint = self.endpoint.clone();
		let store = self.store.clone();
		let job_seq = self.job_seq.clone();

		async move {
			// 1) AuthZ by NodeId
			let peer = conn.remote_node_id().map_err(AcceptError::from_err)?;
			if !allow_node_ids.contains(&peer) {
				error!(%peer, "peer not allowed");
				return Err(AcceptError::from_err(ControlError::Unauthorized));
			}

			// 2) Bi-stream: read request with explicit size limit
			let (mut send, mut recv) = conn.accept_bi().await.map_err(AcceptError::from_err)?;
			let request_buf = recv.read_to_end(CONTROL_MAX_BYTES).await.map_err(|e| {
				AcceptError::from_err(ControlError::Read {
					source: Box::new(e),
				})
			})?;

			let begin: BeginBackup = serde_json::from_slice(&request_buf).map_err(|e| {
				AcceptError::from_err(ControlError::Parse {
					source: e,
				})
			})?;
			debug!(device = %begin.device_tag, tickets = begin.tickets.len(), "begin");

			// 3) Download using FsStore downloader API
			let job_id = job_seq.fetch_add(1, Ordering::Relaxed);

			let mut downloaded = 0usize;
			let mut failed = 0usize;

			let downloader = store.downloader(&endpoint);
			for t in &begin.tickets {
				match t.parse::<BlobTicket>() {
					Ok(ticket) => {
						let provider = Some(ticket.node_addr().node_id);
						match downloader.download(ticket.hash(), provider).await {
							Ok(()) => downloaded += 1,
							Err(e) => {
								failed += 1;
								error!(%e, %job_id, "download failed");
							}
						}
					}
					Err(e) => {
						failed += 1;
						error!(ticket = %t, %e, "ticket parse failed");
					}
				}
			}

			// 4) Reply
			let ack = CompletionAck {
				job_id,
				ok: failed == 0,
				downloaded,
				failed,
				error: (failed > 0).then(|| format!("{failed} failures")),
			};

			let bytes = serde_json::to_vec(&ack).map_err(|e| {
				let io = std::io::Error::other(e);
				AcceptError::from_err(ControlError::Reply {
					source: io,
				})
			})?;

			send.write_all(&bytes).await.map_err(|e| {
				AcceptError::from_err(ControlError::Reply {
					source: e.into(),
				})
			})?;
			// finish() is sync in this API.
			send.finish().map_err(|e| {
				AcceptError::from_err(ControlError::Reply {
					source: e.into(),
				})
			})?;

			info!(job_id, downloaded, failed, "completed");
			Ok(())
		}
	}
}

use std::{env, str::FromStr, sync::Arc};

use iroh::{protocol::RouterBuilder, Endpoint};
use iroh_blobs::{store::fs::FsStore, BlobsProtocol, ALPN as BLOBS_ALPN};
use proto_control::{ControlHandler, CONTROL_ALPN};
use snafu::Snafu;
use tracing::info;
use tracing_subscriber::EnvFilter;

// Errors
#[derive(Debug, Snafu)]
enum MainError {
	#[snafu(display("failed to bind endpoint: {details}"))]
	Bind {
		details: String,
	},

	#[snafu(display("failed to load FsStore at {path}: {details}"))]
	LoadStore {
		path: String,
		details: String,
	},

	#[snafu(display("signal wait failed: {source}"))]
	Signal {
		source: std::io::Error,
	},

	#[snafu(display("router shutdown failed: {details}"))]
	RouterShutdown {
		details: String,
	},
}

async fn init_endpoint() -> Result<Endpoint, MainError> {
	Endpoint::builder().discovery_n0().bind().await.map_err(|e| MainError::Bind {
		details: e.to_string(),
	})
}

async fn load_fs_store(path: &str) -> Result<FsStore, MainError> {
	FsStore::load(path).await.map_err(|e| MainError::LoadStore {
		path: path.to_string(),
		details: e.to_string(),
	})
}

// put this anywhere above main()
fn parse_allow_list(csv: Option<String>) -> std::collections::HashSet<iroh::NodeId> {
	csv.map(|v| v.split(',').filter_map(|s| iroh::NodeId::from_str(s.trim()).ok()).collect())
		.unwrap_or_default()
}

// Main
#[tokio::main]
async fn main() -> Result<(), MainError> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env()) // RUST_LOG=homenect=debug,iroh=info
		.with_target(false)
		.compact()
		.init();

	let store_path =
		env::var("HOMENECT_STORE_PATH").unwrap_or_else(|_| "/srv/homenect/store".to_string());
	let allow = parse_allow_list(env::var("HOMENECT_ALLOW_NODE_IDS").ok());

	let endpoint = init_endpoint().await?;
	let endpoint_arc = Arc::new(endpoint.clone());

	let fs_store = Arc::new(load_fs_store(&store_path).await?);

	// Register blobs protocol for data-path
	let blobs = BlobsProtocol::new(&fs_store, endpoint.clone(), None);

	let router = {
		let builder: RouterBuilder = iroh::protocol::Router::builder(endpoint.clone());
		builder
			.accept(BLOBS_ALPN, blobs.clone())
			// ControlHandler expects Arc<Endpoint> and Arc<FsStore>
			.accept(CONTROL_ALPN.as_bytes(), ControlHandler::new(allow, endpoint_arc, fs_store))
			.spawn()
	};

	info!(node_id = %endpoint.node_id(), "server started");
	tokio::signal::ctrl_c().await.map_err(|source| MainError::Signal {
		source,
	})?;
	router.shutdown().await.map_err(|e| MainError::RouterShutdown {
		details: e.to_string(),
	})?;
	Ok(())
}

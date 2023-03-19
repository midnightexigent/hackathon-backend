use anyhow::{anyhow, Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, system_transaction};
use std::{
    env,
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{debug, info};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Vendor {
    wallet_id: String,
    name: String,
    #[serde(default)]
    address: String,
    #[serde(default)]
    services: Vec<String>,
}

#[derive(Clone)]
struct BuyState {
    vendors: SharedVendors,
    client: Arc<RpcClient>,
}

type SharedVendors = Arc<RwLock<Vec<Vendor>>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let vendors = SharedVendors::default();
    let buy_state = BuyState {
        client: RpcClient::new("https://api.devnet.solana.com").into(),
        vendors: vendors.clone(),
    };
    let app = Router::new()
        .route("/vendors", get(list).post(insert).with_state(vendors))
        .route("/buy", post(buy).with_state(buy_state));

    let port = env::args()
        .nth(1)
        .as_deref()
        .map(u16::from_str)
        .transpose()
        .context("failed to parse provided port")?
        .unwrap_or(3030);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    debug!("listening on {}", addr);
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

struct Error(anyhow::Error);

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self(anyhow::Error::new(value))
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        #[derive(Serialize)]
        struct S {
            error: String,
        }
        let body = Json(S {
            error: self.0.to_string(),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

#[derive(Debug, Deserialize)]
struct BuyParams {
    lamports: u64,
    vendor: String,
    buyer_pair: String,
}

#[tracing::instrument(skip(state))]
async fn buy(State(state): State<BuyState>, Json(params): Json<BuyParams>) -> Result<(), Error> {
    if !state
        .vendors
        .read()
        .unwrap()
        .iter()
        .any(|v| v.wallet_id == params.vendor)
    {
        return Err(Error(anyhow!("{} is not whitelisted", params.vendor)));
    }

    let from_keypair = Keypair::from_base58_string(&params.buyer_pair);
    let to = Pubkey::from_str(&params.vendor)?;
    let sig = state
        .client
        .send_and_confirm_transaction(&system_transaction::transfer(
            &from_keypair,
            &to,
            params.lamports,
            state.client.get_latest_blockhash()?,
        ))?;
    while !state.client.confirm_transaction(&sig)? {}
    Ok(())
}

#[tracing::instrument(skip(vendors))]
async fn list(State(vendors): State<SharedVendors>) -> Json<Vec<Vendor>> {
    info!("retrieving all vendors");
    Json(vendors.read().unwrap().clone())
}

#[tracing::instrument(skip(vendors))]
async fn insert(State(vendors): State<SharedVendors>, Json(input): Json<Vendor>) {
    info!("adding new vendor");
    vendors.write().unwrap().push(input)
}

use axum::{Json, Router, extract::State};
use chain_arbitro::{chain_arbitro::ChainArbitro, manager::ArbitroManager};
use chains::{
    chain::generate_fallback_provider,
    chain_json_model::ChainDataJsonModelSmall,
    chain_sled_model::{AnyPoolConfig, AnyPoolState, V3PoolKey},
    chains::Chains,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

type SDB = Arc<RwLock<Chains>>; //shared db

#[tokio::main]
async fn main() {
    let index_path = "/home/pdr/Arbitro/frontend/dist";
    let assets_path = format!("{}/assets", index_path);
    let serve_dir = tower_http::services::ServeDir::new(index_path);
    let serve_assets = tower_http::services::ServeDir::new(assets_path);

    let chains = Arc::new(RwLock::new(chains::chains::Chains::default()));

    let mut live_chains = HashMap::new();

    let app = Router::new()
        .route_service("/", serve_dir)
        .nest_service("/assets", serve_assets)
        .route("/{id}", axum::routing::get(get_chain_data))
        .route("/chains", axum::routing::get(get_available_chains))
        .with_state(chains);

    let socket_addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    axum_server::bind(socket_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_chain_data(
    axum::extract::Path(chain_id): axum::extract::Path<u64>,
    State(chains): State<SDB>,
) -> Result<axum::Json<ChainDataJsonModelSmall>, ()> {
    if let Some(current_chain) = chains.read().await.get_chain_data(chain_id).await {
        println!("chain {} found", chain_id);
        Ok(Json(current_chain))
    } else {
        println!("chain {} not found, returning empty data", chain_id);
        Err(())
    }
}

async fn get_available_chains(State(chains): State<SDB>) -> axum::Json<Vec<u64>> {
    let chains_idx = chains.read().await.chains.keys().cloned().collect();
    Json(chains_idx)
}

async fn request_paths(State(chains): State<SDB>, tokens: Vec<String>, chain_id: u64) {
    if let Some(chain) = chains.read().await.chains.get(&chain_id) {}
}

async fn post_pool_data(State(_chains): State<SDB>, chain_id: u64, pool_id: PoolKey) {
    let mut chains = _chains.read().await;

    if let Some(chain) = chains.chains.get(&chain_id) {}
}

async fn _test() -> &'static str {
    "a"
}

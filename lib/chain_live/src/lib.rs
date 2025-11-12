use std::{num::NonZeroUsize, str::FromStr};

use alloy::{rpc::client::RpcClient, transports::http::Http};
use alloy_provider::{ProviderBuilder, transport::layers::FallbackLayer};
use tower::ServiceBuilder;
use url::Url;

pub mod factory;

pub mod any_factory;
pub mod any_pool;
pub mod clpool;
pub mod token;
pub mod v2_factory;
pub mod v2_pool;
pub mod v3_factory;
pub mod v3_pool;
pub mod v4_factory;
pub mod v4_pool;

pub type MyProvider = alloy_provider::fillers::FillProvider<
    alloy_provider::fillers::JoinFill<
        alloy_provider::Identity,
        alloy_provider::fillers::JoinFill<
            alloy_provider::fillers::GasFiller,
            alloy_provider::fillers::JoinFill<
                alloy_provider::fillers::BlobGasFiller,
                alloy_provider::fillers::JoinFill<
                    alloy_provider::fillers::NonceFiller,
                    alloy_provider::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy_provider::RootProvider,
>;

pub fn generate_fallback_provider(urls: Vec<String>) -> Option<MyProvider> {
    if urls.is_empty() {
        return None;
    }

    let mut trnsports = Vec::new();
    for _url in &urls {
        if let Ok(url) = Url::from_str(_url) {
            let http = Http::new(url);
            trnsports.push(http);
        }
    }

    let count = NonZeroUsize::try_from(trnsports.len()).unwrap();
    let fallback_layer = FallbackLayer::default().with_active_transport_count(count);
    let service = ServiceBuilder::new()
        /*.layer(DBLayer {
            db: chain_db.unwrap(),
        })*/
        .layer(fallback_layer)
        .service(trnsports);

    let client = RpcClient::builder().transport(service, false);

    Some(ProviderBuilder::new().connect_client(client))
}

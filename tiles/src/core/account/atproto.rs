//! Handles atprotocol stuff

use std::sync::Arc;

use anyhow::Result;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig, DnsTxtResolver},
};
use atrium_oauth::{
    AtprotoLocalhostClientMetadata, AuthorizeOptions, DefaultHttpClient, KnownScope, OAuthClient,
    OAuthClientConfig, OAuthResolverConfig, Scope,
    store::{session::MemorySessionStore, state::MemoryStateStore},
};

use hickory_resolver::TokioResolver;
use std::error::Error;

use hickory_resolver::Resolver;
use hickory_resolver::config::*;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use tokio::runtime::Runtime;

struct HickoryDnsTxtResolver {
    resolver: TokioResolver,
}

impl Default for HickoryDnsTxtResolver {
    fn default() -> Self {
        Self {
            resolver: build_resolver(),
        }
    }
}

impl DnsTxtResolver for HickoryDnsTxtResolver {
    async fn resolve(
        &self,
        query: &str,
    ) -> core::result::Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
        Ok(self
            .resolver
            .txt_lookup(query)
            .await?
            .answers()
            .iter()
            .map(|txt| txt.to_string())
            .collect())
    }
}

fn build_resolver() -> Resolver<TokioRuntimeProvider> {
    Resolver::builder_with_config(
        ResolverConfig::udp_and_tcp(&GOOGLE),
        TokioRuntimeProvider::default(),
    )
    .build()
    .unwrap()
}
pub async fn login() -> Result<()> {
    let http_client = Arc::new(DefaultHttpClient::default());

    let config = OAuthClientConfig {
        client_metadata: AtprotoLocalhostClientMetadata {
            redirect_uris: Some(vec![String::from("http://127.0.0.1/callback")]),
            scopes: Some(vec![Scope::Known(KnownScope::Atproto)]),
        },
        keys: None,
        resolver: OAuthResolverConfig {
            did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
                http_client: http_client.clone(),
            }),
            handle_resolver: AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                dns_txt_resolver: HickoryDnsTxtResolver::default(),
                http_client: http_client.clone(),
            }),
            authorization_server_metadata: Default::default(),
            protected_resource_metadata: Default::default(),
        },
        state_store: MemoryStateStore::default(),
        session_store: MemorySessionStore::default(),
    };

    let Ok(client) = OAuthClient::new(config) else {
        panic!("client fuck up")
    };

    let url = client
        .authorize(
            "madcla.ws",
            AuthorizeOptions {
                scopes: vec![Scope::Known(KnownScope::Atproto)],
                ..Default::default()
            },
        )
        .await?;

    println!("url\n{}", url);
    Ok(())
}

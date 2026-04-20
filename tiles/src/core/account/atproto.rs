//! Handles atprotocol stuff

use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig, DnsTxtResolver},
};
use atrium_oauth::{
    AtprotoLocalhostClientMetadata, AuthorizeOptions, DefaultHttpClient, KnownScope, OAuthClient,
    OAuthClientConfig, OAuthResolverConfig, Scope,
    store::{session::MemorySessionStore, state::MemoryStateStore},
};
use reqwest::Client;
use serde::Deserialize;

use std::error::Error;

use hickory_resolver::TokioResolver;

#[derive(Deserialize)]
struct HandleResolve {
    did: String,
}

struct HickoryDnsTxtResolver {
    resolver: TokioResolver,
}

impl Default for HickoryDnsTxtResolver {
    fn default() -> Self {
        Self {
            resolver: TokioResolver::builder_tokio()
                .expect("Failed to create TokioResolver builder")
                .build()
                .expect("Failed to build tokio resolver"),
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

pub async fn login(handle: &str) -> Result<()> {
    let http_client = Arc::new(DefaultHttpClient::default());

    let config = OAuthClientConfig {
        client_metadata: AtprotoLocalhostClientMetadata {
            redirect_uris: Some(vec![String::from("http://127.0.0.1:1729/callback")]),
            scopes: Some(vec![
                Scope::Known(KnownScope::Atproto),
                Scope::Known(KnownScope::TransitionGeneric),
            ]),
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

    //TODO: This resolve function is hack to convert handle to DID
    // cuz for some reason the authorize fn not working for customd domains
    // it does work for bluesky hosted handles and DIDs.
    // Probably smthng to do w DNS resolver. Will dig more latta
    let did = resolve_handle_to_did(handle).await?;

    let url = client
        .authorize(
            did,
            AuthorizeOptions {
                scopes: vec![
                    Scope::Known(KnownScope::Atproto),
                    Scope::Known(KnownScope::TransitionGeneric),
                ],
                ..Default::default()
            },
        )
        .await?;

    println!("url\n{}", url);
    Ok(())
}

async fn resolve_handle_to_did(handle: &str) -> Result<String> {
    let client_builder = Client::builder().timeout(Duration::from_secs(5));
    let client = client_builder.build()?;
    let response = client
        .get(format!(
            "https://bsky.social/xrpc/com.atproto.identity.resolveHandle?handle={}",
            handle
        ))
        .send()
        .await;

    match response {
        Err(err) if err.is_timeout() => Err(anyhow!("Request failed due to Api timedout")),
        Err(err) => Err(anyhow!("Request failed due to {:?}", err)),
        Ok(res) if res.status() == 200 => {
            let resolve_data = res.json::<HandleResolve>().await?;
            Ok(resolve_data.did)
        }
        Ok(res) => Err(anyhow!("Api failed with status {}", res.status())),
    }
}

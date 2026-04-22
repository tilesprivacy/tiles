//! Handles atprotocol stuff

use std::{process::Command, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig, DnsTxtResolver},
};
use atrium_oauth::{
    AtprotoLocalhostClientMetadata, AuthorizeOptions, CallbackParams, DefaultHttpClient,
    KnownScope, OAuthClient, OAuthClientConfig, OAuthResolverConfig, Scope,
    store::{session::MemorySessionStore, state::MemoryStateStore},
};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use std::error::Error;

use hickory_resolver::TokioResolver;

use crate::daemon::start_internal_server;

#[derive(Deserialize)]
struct HandleResolve {
    did: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AtCallbackParams {
    code: Option<String>,
    iss: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
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
    const LOGIN_PORT: u32 = 8988;
    let config = OAuthClientConfig {
        client_metadata: AtprotoLocalhostClientMetadata {
            redirect_uris: Some(vec![String::from("http://127.0.0.1:8988/callback")]),
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
    let did = resolve_handle_to_did(handle)
        .await
        .inspect_err(|_| eprintln!("Failed to resolve handle"))?;

    info!("{}", did);
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
        .await
        .inspect_err(|_| eprintln!("Failed to authorize"))?;

    println!("url\n{}", url);
    let mut child = Command::new("open").arg(url).spawn()?;
    child.wait()?;
    let (callback_tx, callback_rx) = oneshot::channel();

    //TODO: can we randomze port
    start_internal_server(Some(LOGIN_PORT), callback_tx).await?;
    let params = callback_rx.await?;
    info!("params recieved {:?}", params);

    if let Some(code) = params.code {
        let cb_params = CallbackParams {
            code,
            state: params.state,
            iss: params.iss,
        };
        let auth_session = client.callback(cb_params).await?;
        // This session will be stored in the Memstore
        // SO before doing authorize, we try to restore a
        // session by the DID
        // Need to implement a SessionStore on sqlite table
    } else {
        eprintln!(
            "Error authorizing due to {}",
            params
                .error_description
                .unwrap_or("unknow reason".to_owned())
        )
    }
    // wait for the server to send params
    // once that's done we can create oauthsession
    // shut the serve too once its done.
    // add it to DB
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

//! The main module for networking

pub mod ticket;
use std::{collections::HashMap, str::FromStr, time::Duration};

use anyhow::{Context, Result, anyhow};
use axum::body::Bytes;
use futures_util::{StreamExt, TryStreamExt};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, NET_REPORT_TIMEOUT, PublicKey, Signature,
    endpoint::{BindError, presets},
    endpoint_info::UserData,
    protocol::Router,
};
use iroh_blobs::{BlobsProtocol, store::mem::MemStore, ticket::BlobTicket};
use iroh_gossip::{
    Gossip, TopicId,
    api::{Event, GossipReceiver, GossipSender, GossipTopic},
};
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use iroh_tickets::endpoint::EndpointTicket;
use log::info;

use tokio::{
    io::{AsyncWriteExt, copy},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Sender},
        oneshot::{self, Receiver as oneshotReceiver},
    },
};
use uuid::Uuid;

use crate::{
    core::{
        account::{
            self, get_did_from_public_key, get_public_key_from_did, get_random_bytes,
            local::{
                Capability, Token, TokenType, User, add_token, create_invocation_token,
                create_token, fetch_delegated_tokens, get_app_secret_key, get_current_user,
                parse_invocation_token, verify_invocation,
            },
        },
        chats::{SyncOp, create_db_channel, get_last_row_counter},
        network::ticket::EndpointUserData,
        storage::db::{DBTYPE, get_db_conn},
    },
    daemon::net::{API_TIMEOUT, NetworkApiEvent},
    utils::{
        crypto::{
            ContentKey, create_kem_pair, decrypt_by_content_key, decrypt_by_hpke,
            encrypt_by_content_key, encrypt_by_hpke,
        },
        get_unix_time_now,
    },
};
use owo_colors::OwoColorize;
use sha2::{Digest, Sha256};

// 50 mb
const MAX_DOWNLOADED_BYTES: usize = 50 * 1024 * 1024;
const MAX_MESSAGE_AGE_MILLI_SECS: u64 = 5 * 60 * 1000;
const MAX_REPLAY_CACHE_ENTRIES: usize = 10_000;

const ALPN: &[u8] = b"remote-link/v1";

const HPKE_SYNC_DOMAIN_SEPARATION: &str = "sync-hpke/v1";

// const DEVICE_LINK_LOCAL_TOPIC: &str = "com.tilesprivacy.tiles.link";
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct NetworkMessage {
    from_did: String,
    from_nickname: String,
    to_did: String,
    is_online: bool,
    body: MessageBody,
    // to prevent iroh's deduplication on same msg
    nonce: [u8; 16],
    // unix milliseconds, signed together with the message
    timestamp: u64,
    msg_type: NetworkMessageType,
}

impl NetworkMessage {
    fn new(
        user: &account::local::User,
        is_online: bool,
        body: MessageBody,
        msg_type: NetworkMessageType,
        to_did: String,
    ) -> Self {
        Self {
            from_did: user.user_id.clone(),
            from_nickname: user.username.clone().unwrap_or("".to_string()),
            is_online,
            body,
            nonce: get_random_bytes(),
            timestamp: get_unix_time_now(),
            msg_type,
            to_did,
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect("Failed to convert to bytes w postcard")
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SignedNetworkMessage {
    msg: NetworkMessage,
    signature: Signature,
}

impl SignedNetworkMessage {
    fn sign(msg: NetworkMessage) -> Result<Self> {
        let secret_key = get_app_secret_key(&msg.from_did)?;
        Ok(Self::sign_with_secret_key(msg, &secret_key))
    }

    fn sign_with_secret_key(msg: NetworkMessage, secret_key: &iroh::SecretKey) -> Self {
        let signature = secret_key.sign(&msg.to_bytes());

        Self { msg, signature }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("Failed to convert signed message to bytes w postcard")
    }

    fn verify(&self) -> Result<()> {
        let public_key = self.sender_public_key()?;
        self.verify_with_public_key(&public_key)
    }

    fn sender_public_key(&self) -> Result<PublicKey> {
        let public_key_bytes = get_public_key_from_did(&self.msg.from_did)?;
        Ok(PublicKey::from_bytes(&public_key_bytes)?)
    }

    fn verify_with_public_key(&self, public_key: &PublicKey) -> Result<()> {
        public_key.verify(&self.msg.to_bytes(), &self.signature)?;
        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq)]
struct ReplayKey {
    from_did: String,
    nonce: [u8; 16],
}

struct ReplayCache {
    seen: HashMap<ReplayKey, u64>,
}

impl ReplayCache {
    fn new() -> Self {
        Self {
            seen: HashMap::new(),
        }
    }

    fn accept(&mut self, from_did: &str, nonce: [u8; 16], now: u64) -> bool {
        self.seen.retain(|_, expires_at| *expires_at > now);

        let key = ReplayKey {
            from_did: from_did.to_owned(),
            nonce,
        };

        if self.seen.contains_key(&key) || self.seen.len() >= MAX_REPLAY_CACHE_ENTRIES {
            return false;
        }

        self.seen.insert(key, now + MAX_MESSAGE_AGE_MILLI_SECS);
        true
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum NetworkMessageType {
    Link,
    Sync,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[allow(clippy::enum_variant_names)]
enum MessageBody {
    LinkRequest {
        auth_token: String,
        peer_delegation_token: String,
        nickname: Option<String>,
    },
    LinkAccepted,
    LinkRejected {
        reason: String,
    },
    SyncStart {
        last_row_counter: Option<i64>,
        auth_token: String,
        session_id: String,
        // ephemeral KEM pubkey for the session
        pub_key: Vec<u8>,
    },
    SyncSendDeltaInfo {
        blob_ticket: String,
        last_row_counter: Option<i64>,
        auth_token: String,
        content_key: Vec<u8>,
        pub_key: Vec<u8>,
        encapped_key: Vec<u8>,
        session_id: String,
    },
    SyncEnd {
        auth_token: String,
        session_id: String,
    },
    SyncRejected {
        reason: String,
        auth_token: String,
    },
}

#[derive(Clone)]
pub struct SyncAcceptLoopData {
    user: User,
    store: MemStore,
    endpoint: Endpoint,
    db_channel_sender: Sender<SyncOp>,
    // Map of local ephemeral key-pair of a sync session <sessionId, (secret_key, pub_key) pair>
    sync_sessions: HashMap<String, (Vec<u8>, Vec<u8>)>,
}

/// At onces there can be multiple initiator gossip loop processes but only
/// one Listener loop. This flag for each process will help us in which gossip
/// events should be ignored and not
enum GossipProcessType {
    Listener,
    Initiator,
}

/// Codes used inside the accept gossip loop that decides the exit or contination of the loop.
/// This is useful to shutdown request based spawned process_loops, so that the gossip recievers goes out of scope and automatically disconnect from the topic
enum GossipLoopCode {
    Exit,
    Continue,
}

enum DeviceLinkSelector {
    Error(String),
    Joined(GossipTopic),
}
/// Handles the iroh gossip events
async fn process_gossip_network_events(
    mut msg_receiver: GossipReceiver,
    msg_sender: GossipSender,
    user: account::local::User,
    sync_helpers: Option<SyncAcceptLoopData>,
    caller_sender: tokio::sync::mpsc::Sender<NetworkApiEvent>,
    process_type: GossipProcessType,
) -> Result<()> {
    let mut replay_cache = ReplayCache::new();

    while let Some(event) = msg_receiver.try_next().await? {
        info!(
            "Gossip Loop: Received by {:?}:, event {:?}",
            user.username, event
        );
        if let Event::Received(msg) = event {
            let signed_msg = match SignedNetworkMessage::from_bytes(&msg.content) {
                Ok(msg) => msg,
                Err(err) => {
                    log::error!("Failed to decode msg due to {:?}", err);
                    continue;
                }
            };

            if let Err(err) = signed_msg.verify() {
                log::warn!(
                    "Rejected network message from {}: {:?}",
                    signed_msg.msg.from_did,
                    err
                );
                continue;
            }

            let now = get_unix_time_now();
            if signed_msg.msg.timestamp.abs_diff(now) > MAX_MESSAGE_AGE_MILLI_SECS {
                log::warn!(
                    "Rejected stale network message from {}",
                    signed_msg.msg.from_did
                );
                continue;
            }

            if signed_msg.msg.to_did != user.user_id {
                log::debug!(
                    "Ignoring network message intended for {} instead of {}",
                    signed_msg.msg.to_did,
                    user.user_id
                );
                continue;
            }

            if !replay_cache.accept(&signed_msg.msg.from_did, signed_msg.msg.nonce, now) {
                log::warn!(
                    "Rejected replayed network message from {}",
                    signed_msg.msg.from_did
                );
                continue;
            }

            match signed_msg.sender_public_key() {
                Ok(public_key) => public_key,
                Err(err) => {
                    log::warn!(
                        "Rejected network message from {} due to invalid sender identity: {:?}",
                        signed_msg.msg.from_did,
                        err
                    );
                    continue;
                }
            };

            let msg = signed_msg.msg;

            let loop_code = match msg.msg_type {
                NetworkMessageType::Link => {
                    match handle_device_link_events(
                        msg,
                        &user,
                        &caller_sender,
                        &msg_sender,
                        &process_type,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(err) => {
                            log::error!("Error handling device event due to {:?}", err);
                            continue;
                        }
                    }
                }
                NetworkMessageType::Sync => {
                    if let Some(sync_helpers) = sync_helpers.clone() {
                        match handle_sync_events(
                            msg,
                            sync_helpers,
                            &caller_sender,
                            &msg_sender,
                            &process_type,
                        )
                        .await
                        {
                            Ok(res) => res,
                            Err(err) => {
                                log::error!("Error handling sync events due to {:?}", err);
                                continue;
                            }
                        }
                    } else {
                        log::error!("No sync helper passed, critical error exiting");
                        GossipLoopCode::Exit
                    }
                }
            };

            // exiting the accept loop, thus removing thyself from the topic on which this process was spawned
            if matches!(loop_code, GossipLoopCode::Exit) {
                info!("Exiting the gossip accepter loop, DID: {}", user.user_id);
                return Ok(());
            }
        }
    }
    Ok(())
}

async fn handle_device_link_events(
    msg: NetworkMessage,
    user: &User,
    caller_sender: &tokio::sync::mpsc::Sender<NetworkApiEvent>,
    msg_sender: &GossipSender,
    process_type: &GossipProcessType,
) -> Result<GossipLoopCode> {
    log::info!("Handling device link requests");
    match msg.body {
        MessageBody::LinkRequest {
            ref auth_token,
            ref peer_delegation_token,
            ref nickname,
        } => {
            log::info!("on Link request");
            if matches!(process_type, GossipProcessType::Initiator) {
                log::warn!("Rejected the Link request on an Initiator process");
                return Ok(GossipLoopCode::Continue);
            }
            if let Err(err) = verify_invocation_token(auth_token, &msg).await {
                let err_reason =
                    format!("link token invocation verification failed due to {:?}", err);
                return handle_device_link_rejection_by_listener(
                    &err_reason,
                    user,
                    &msg,
                    msg_sender,
                )
                .await;
            }

            let conn = get_db_conn(&DBTYPE::COMMON)?;

            if let Err(err) = add_token(peer_delegation_token, &conn, nickname.as_deref()) {
                let err_reason = format!("adding peer delegation token failed due to {:?}", err);

                return handle_device_link_rejection_by_listener(
                    &err_reason,
                    user,
                    &msg,
                    msg_sender,
                )
                .await;
            }

            let link_resp = NetworkMessage::new(
                user,
                msg.is_online,
                MessageBody::LinkAccepted,
                NetworkMessageType::Link,
                msg.from_did,
            );

            let link_resp = SignedNetworkMessage::sign(link_resp)?;
            msg_sender.broadcast(link_resp.to_bytes().into()).await?;
        }
        MessageBody::LinkAccepted => {
            log::info!("On link accepted");
            if matches!(process_type, GossipProcessType::Listener) {
                log::warn!("Rejected the Link Accepted on a listener process");
                return Ok(GossipLoopCode::Continue);
            }
            caller_sender
                .send(NetworkApiEvent::DeviceLinkSuccess)
                .await?;

            return Ok(GossipLoopCode::Exit);
        }

        MessageBody::LinkRejected { reason } => {
            log::info!("On link rejected");
            if matches!(process_type, GossipProcessType::Listener) {
                log::warn!("Rejected the Link Rejected event on a listener process");
                return Ok(GossipLoopCode::Continue);
            }
            caller_sender
                .send(NetworkApiEvent::DeviceLinkFailed(reason))
                .await?;
            return Ok(GossipLoopCode::Exit);
        }
        msg_body => {
            log::warn!("Invalid link message {:?}", msg_body)
        }
    }
    Ok(GossipLoopCode::Continue)
}

async fn handle_sync_events(
    msg: NetworkMessage,
    sync_helpers: SyncAcceptLoopData,
    caller_sender: &tokio::sync::mpsc::Sender<NetworkApiEvent>,
    msg_sender: &GossipSender,
    process_type: &GossipProcessType,
) -> Result<GossipLoopCode> {
    let mut sync_helpers = sync_helpers;
    match msg.body {
        MessageBody::SyncStart { .. } => {
            info!("Received sync start event...");
            if matches!(process_type, GossipProcessType::Initiator) {
                log::warn!("Rejected the SyncStart event on an initiator process");
                return Ok(GossipLoopCode::Continue);
            }
            let senders: (
                tokio::sync::mpsc::Sender<SyncOp>,
                tokio::sync::mpsc::Sender<NetworkApiEvent>,
            ) = (
                sync_helpers.db_channel_sender.clone(),
                caller_sender.clone(),
            );
            on_sync_start_event(msg_sender, &mut sync_helpers, &msg, senders).await
        }
        MessageBody::SyncSendDeltaInfo { .. } => {
            info!("on syncSendDeltaInfo");

            let senders: (
                tokio::sync::mpsc::Sender<SyncOp>,
                tokio::sync::mpsc::Sender<NetworkApiEvent>,
            ) = (
                sync_helpers.db_channel_sender.clone(),
                caller_sender.clone(),
            );
            on_sync_send_delta_info(msg_sender, &mut sync_helpers, &msg, senders).await
        }
        MessageBody::SyncEnd { ref auth_token, .. } => {
            // SyncEnd is received by the initiator
            if matches!(process_type, GossipProcessType::Listener) {
                log::warn!("Rejected the SyncEnd event on an Listener process");
                return Ok(GossipLoopCode::Continue);
            }

            if let Err(err) = verify_invocation_token(auth_token, &msg).await {
                log::warn!("Sync end command failed due to {}", err);
                Ok(GossipLoopCode::Continue)
            } else {
                log::info!("Sync completed..., exiting..");
                caller_sender.send(NetworkApiEvent::SyncSuccess).await?;
                Ok(GossipLoopCode::Exit)
            }
        }
        MessageBody::SyncRejected {
            ref reason,
            ref auth_token,
        } => {
            log::warn!("On sync rejected");
            if matches!(process_type, GossipProcessType::Listener) {
                log::warn!("Rejected the SyncRejected event on an Listener process");
                return Ok(GossipLoopCode::Continue);
            };

            if let Err(err) = verify_invocation_token(auth_token, &msg).await {
                log::warn!("Sync rejected command failed due to {}", err);
                Ok(GossipLoopCode::Continue)
            } else {
                caller_sender
                    .send(NetworkApiEvent::SyncFailed(reason.to_string()))
                    .await?;
                Ok(GossipLoopCode::Exit)
            }
        }
        msg_body => {
            info!("Invalid sync message {:?}", msg_body);
            Ok(GossipLoopCode::Continue)
        }
    }
}

async fn verify_invocation_token(auth_token: &str, msg: &NetworkMessage) -> Result<()> {
    let invocation = parse_invocation_token(auth_token)?;
    if invocation.issuer().as_str() != msg.from_did || invocation.audience().as_str() != msg.to_did
    {
        return Err(anyhow!("Invocation token not belong to sender"));
    }
    verify_invocation(invocation).await
}

async fn create_auth_invocation_token(
    subject_did: &str,
    common_conn: tokio_rusqlite::rusqlite::Connection,
) -> Result<String> {
    let delegated_tokens = fetch_delegated_tokens(subject_did, &common_conn, None)?;

    create_invocation_token(
        delegated_tokens.iter().map(|t| t.token.as_str()).collect(),
        common_conn,
    )
    .await
}
/// Creats an Iroh endpoint. All the tiles communication references this endpoint
pub async fn create_endpoint(user: &account::local::User) -> Result<Endpoint> {
    info!("Creating endpoint for {}", user.user_id);
    // In release mode, we will build the endpoint using
    // tiles keypair in keychain
    let usr_data = EndpointUserData::new(
        &user.user_id,
        user.username.as_ref().unwrap_or(&user.user_id),
    );
    if !cfg!(debug_assertions) {
        let secret_key = get_app_secret_key(&user.user_id)?;
        Endpoint::builder(presets::N0)
            .user_data_for_address_lookup(UserData::try_from(usr_data.to_string())?)
            .secret_key(secret_key)
            .bind()
            .await
            .map_err(<BindError as Into<anyhow::Error>>::into)
    } else {
        Endpoint::builder(presets::N0)
            .user_data_for_address_lookup(UserData::try_from(usr_data.to_string())?)
            .bind()
            .await
            .map_err(<BindError as Into<anyhow::Error>>::into)
    }
}

/// Starts tiles network in background
///
/// All p2p communcation goes thru this network
pub async fn start_network(
    user: crate::core::account::local::User,
    endpoint: Endpoint,
    caller_resp_sender: tokio::sync::mpsc::Sender<NetworkApiEvent>,
) -> Result<()> {
    let did = if cfg!(debug_assertions) {
        let pub_key = endpoint.id();
        &get_did_from_public_key(pub_key.as_bytes())?
    } else {
        &user.user_id
    };

    // sender/recv for cross process communication with the tiles-network
    let (main_network_sender, main_network_recv) = mpsc::channel::<NetworkApiEvent>(10);

    let network_topic = format!("tiles:{}", did);
    let network_topic_id = create_topic_id(&network_topic);
    let (gossip, router_recv, store) = create_tiles_gossip_endpoint(&endpoint).await?;

    let (gossip_network_sender, gossip_network_receiver) =
        gossip.subscribe(network_topic_id, vec![]).await?.split();

    info!("Started tiles gossip network for {}", user.user_id);

    let db_channel_sender = create_db_channel();

    let sync_helpers = SyncAcceptLoopData {
        user: user.clone(),
        store,
        endpoint: endpoint.clone(),
        db_channel_sender: db_channel_sender.clone(),
        sync_sessions: HashMap::new(),
    };

    tokio::spawn(process_gossip_network_events(
        gossip_network_receiver,
        gossip_network_sender.clone(),
        user.clone(),
        Some(sync_helpers),
        main_network_sender.clone(),
        GossipProcessType::Listener,
    ));

    info!("{}", "Ready to accept requests from peers...".blue());
    caller_resp_sender
        .send(NetworkApiEvent::NetworkStarted((
            gossip,
            main_network_sender.clone(),
        )))
        .await?;

    // Since in dev, we create endpoints randomly, at the initiator side
    // we can use the DID derived from this, instead of actual ones
    // for the network to form correctly
    if cfg!(debug_assertions) {
        println!("Use this DID {} in dev for testing", did);
    };

    let mut main_network_recv = main_network_recv;

    // such that the main listener should only exit on StopNetwork event
    while let Some(network_api_event) = main_network_recv.recv().await {
        match network_api_event {
            NetworkApiEvent::StopNetwork => {
                router_recv.shutdown().await?;
                endpoint.close().await;
                info!("Closing the tiles network listener");
                break;
            }
            event => log::info!(
                "Noop for the event {:?} on the main listener process",
                event
            ),
        }
    }

    Ok(())
}

/// Initiates the device linking with the delegated token
///
/// ## The device-linking process
/// Since the device links are with your own devices. All devices have same access. For partial access they don't need device linking.
///Since UCAN is one-way delegation of access. The parent device A delegates full access to device B.
/// But the general UX for linking is that once linked both devices has bidirectional capability cue Syncthing, perplexity et.al
/// So to achieve this UX, the device B which `join` the device A with delegated token, internaly creates a reverse ucan token that delegates A all the capabilities of B and send during this process.
/// So once done, both are linked peers and has seamless bi-directional communication ability
pub async fn initiate_device_linking(link_token: &str, gossip: Gossip) -> Result<()> {
    let conn = get_db_conn(&DBTYPE::COMMON)?;
    let user = get_current_user(&conn)?;

    let delegation_token = Token::parse_ucan_delegation(link_token)?;

    let delegated_tokens =
        fetch_delegated_tokens(&delegation_token.aud_did, &conn, Some(TokenType::Linked))?;

    // Checking if this device already has a powerline ucan token to the issuer. If yes, send it. Else create a reverse ucan token that delegates all the caps to A
    let peer_delegation_token = if let Some(issuer_delegated_token) = delegated_tokens
        .into_iter()
        .find(|t| t.aud_did == delegation_token.did)
    {
        issuer_delegated_token
    } else {
        // create a reverse delegated token
        create_token(
            &delegation_token.did,
            user.username.as_deref(),
            Capability::All,
        )
        .await?
    };

    // create invocation token
    let invocation_token = create_invocation_token(vec![link_token], conn).await?;

    let device_link_topic = format!("tiles:{}", delegation_token.did);
    let device_link_topic_id = create_topic_id(&device_link_topic);

    let issuer_pub_key = get_public_key_from_did(&delegation_token.did)?;

    let issuer_endpoint_id = PublicKey::from_bytes(&issuer_pub_key)?;
    info!("issuer endpoint id {:?}", issuer_endpoint_id);

    let selector = tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(API_TIMEOUT as u64)) => {
            DeviceLinkSelector::Error("Timeout while waiting for the issuer's endpoint".to_string())
        }
       gossip_topic_result = gossip.subscribe_and_join(device_link_topic_id, vec![issuer_endpoint_id]) => {
            if let Ok(gossip_topics) = gossip_topic_result{
                DeviceLinkSelector::Joined(gossip_topics)
            } else {
                DeviceLinkSelector::Error("No gossip topic found".to_string())
            }
        }
    };

    match selector {
        DeviceLinkSelector::Error(reason) => Err(anyhow!(reason)),
        DeviceLinkSelector::Joined(gossip_topic) => {
            let (gossip_network_sender, gossip_network_receiver) = gossip_topic.split();
            let (caller_sender, mut caller_recv) = mpsc::channel::<NetworkApiEvent>(10);

            let handle = tokio::spawn(process_gossip_network_events(
                gossip_network_receiver,
                gossip_network_sender.clone(),
                user.clone(),
                None,
                caller_sender.clone(),
                GossipProcessType::Initiator,
            ));

            let link_req_msg = NetworkMessage::new(
                &user,
                true,
                MessageBody::LinkRequest {
                    auth_token: invocation_token,
                    peer_delegation_token: peer_delegation_token.token,
                    nickname: user.username.clone(),
                },
                NetworkMessageType::Link,
                delegation_token.did.to_string(),
            );
            let link_req_msg = SignedNetworkMessage::sign(link_req_msg)?;
            gossip_network_sender
                .broadcast(link_req_msg.to_bytes().into())
                .await
                .inspect_err(|_e| {
                    handle.abort();
                })?;

            let resp = match tokio::time::timeout(
                Duration::from_secs(API_TIMEOUT as u64),
                caller_recv.recv(),
            )
            .await
            {
                Ok(Some(NetworkApiEvent::DeviceLinkFailed(reason))) => Err(anyhow!(reason)),
                Ok(Some(NetworkApiEvent::DeviceLinkSuccess)) => Ok(()),
                Ok(Some(_)) => Err(anyhow!("Invalid event")),
                Ok(None) => Err(anyhow!("Network process closed unexpectedly")),
                Err(_err) => Err(anyhow!("Timed out waiting for linking response")),
            };
            handle.abort();
            resp
        }
    }
}

/// Initiate sync with a peer
pub async fn sync(did: &str, gossip: Gossip, endpoint: Endpoint) -> Result<()> {
    let common_conn = get_db_conn(&DBTYPE::COMMON)?;
    let user = get_current_user(&common_conn)?;
    let device_link_topic = format!("tiles:{}", did);
    let device_link_topic_id = create_topic_id(&device_link_topic);

    let issuer_pub_key = get_public_key_from_did(did)?;

    let issuer_endpoint_id = PublicKey::from_bytes(&issuer_pub_key)?;
    info!("issuer endpoint id {:?}", issuer_endpoint_id);

    let selector = tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(API_TIMEOUT as u64)) => {
            DeviceLinkSelector::Error("Timeout while waiting for the issuer's endpoint".to_string())
        }
       gossip_topic_result = gossip.subscribe_and_join(device_link_topic_id, vec![issuer_endpoint_id]) => {
            if let Ok(gossip_topics) = gossip_topic_result{
                DeviceLinkSelector::Joined(gossip_topics)
            } else {
                DeviceLinkSelector::Error("No gossip topic found".to_string())
            }
        }
    };

    match selector {
        DeviceLinkSelector::Error(reason) => Err(anyhow!(reason)),
        DeviceLinkSelector::Joined(gossip_topic) => {
            let (gossip_network_sender, gossip_network_receiver) = gossip_topic.split();
            let (caller_sender, mut caller_recv) = mpsc::channel::<NetworkApiEvent>(10);

            let invocation_token = create_auth_invocation_token(did, common_conn).await?;

            let chat_conn = get_db_conn(&DBTYPE::CHAT)?;
            let last_chat_counter = get_last_row_counter(&chat_conn, did)?;

            let db_channel_sender = create_db_channel();

            let sync_session_id: String = Uuid::now_v7().into();

            let session_keypair_tuple = create_kem_pair();
            let mut sync_sessions = HashMap::new();
            sync_sessions.insert(sync_session_id.clone(), session_keypair_tuple.clone());

            let sync_helpers = SyncAcceptLoopData {
                user: user.clone(),
                store: MemStore::new(),
                endpoint,
                db_channel_sender,
                sync_sessions,
            };
            let handle = tokio::spawn(process_gossip_network_events(
                gossip_network_receiver,
                gossip_network_sender.clone(),
                user.clone(),
                Some(sync_helpers),
                caller_sender.clone(),
                GossipProcessType::Initiator,
            ));

            let sync_start_msg = NetworkMessage::new(
                &user,
                true,
                MessageBody::SyncStart {
                    last_row_counter: Some(last_chat_counter),
                    auth_token: invocation_token,
                    session_id: sync_session_id.clone(),
                    pub_key: session_keypair_tuple.1,
                },
                NetworkMessageType::Sync,
                did.to_string(),
            );
            let sync_start_msg = SignedNetworkMessage::sign(sync_start_msg)?;
            gossip_network_sender
                .broadcast(sync_start_msg.to_bytes().into())
                .await
                .inspect_err(|_e| {
                    handle.abort();
                })?;

            let resp = match tokio::time::timeout(
                Duration::from_secs(API_TIMEOUT as u64),
                caller_recv.recv(),
            )
            .await
            {
                Ok(Some(NetworkApiEvent::SyncFailed(reason))) => Err(anyhow!(reason)),
                Ok(Some(NetworkApiEvent::SyncSuccess)) => Ok(()),
                Ok(Some(_)) => Err(anyhow!("Invalid event")),
                Ok(None) => Err(anyhow!("Network process closed unexpectedly")),
                Err(_err) => Err(anyhow!("Timed out waiting for syncing response")),
            };
            handle.abort();
            resp
        }
    }
}

/// Router with gossip and blob protocol
pub(crate) async fn create_tiles_gossip_endpoint(
    endpoint: &Endpoint,
) -> Result<(Gossip, Router, MemStore)> {
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let store = MemStore::new();
    let blobs = BlobsProtocol::new(&store, None);
    let recv_router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .accept(iroh_blobs::ALPN, blobs.clone())
        .spawn();

    Ok((gossip, recv_router, store))
}

fn create_topic_id(topic_name: &str) -> TopicId {
    let mut hasher = Sha256::new();
    hasher.update(topic_name.as_bytes());
    let topic_id_bytes = hasher.finalize();
    TopicId::from_bytes(topic_id_bytes.into())
}

fn _get_did_from_endpoint(endpoint_id: EndpointId) -> Result<String> {
    get_did_from_public_key(endpoint_id.as_bytes())
}

async fn _is_online(endpoint: &Endpoint) -> bool {
    tokio::select! {
        _ = endpoint.online() => {
            true
        }
        _ = tokio::time::sleep(Duration::from_secs(NET_REPORT_TIMEOUT)) => {
            false
        }
    }
}

// As of now we exit asap when we see a peer. This is subjected to change
// as the scale
async fn _find_offline_bootstrap_peers(
    endpoint: &Endpoint,
    mdns: MdnsAddressLookup,
) -> Result<(Vec<EndpointId>, UserData)> {
    let mut bootstrap_ids: Vec<EndpointId> = vec![];
    endpoint.address_lookup()?.add(mdns.clone());
    let mut mdns_event = mdns.subscribe().await;
    let mut user_data = UserData::from_str("")?;
    while let Some(event) = mdns_event.next().await {
        match event {
            DiscoveryEvent::Discovered {
                endpoint_info,
                last_updated: _,
            } => {
                if cfg!(debug_assertions) {
                    println!("peer discoverd {:?}", endpoint_info);
                }
                bootstrap_ids.push(endpoint_info.endpoint_id);
                user_data = endpoint_info.user_data().unwrap().clone();
                break;
            }
            DiscoveryEvent::Expired { endpoint_id } => {
                if cfg!(debug_assertions) {
                    println!("peer left {:?}", endpoint_id)
                }
            }
            _ => {}
        }
    }

    Ok((bootstrap_ids, user_data))
}

async fn fetch_last_row_counter(
    user_id: &str,
    sender: &tokio::sync::mpsc::Sender<SyncOp>,
) -> Result<i64> {
    let (sendx, recvx) = oneshot::channel();
    let sync_op_msg = SyncOp::GetLastRowCounter {
        user_id: user_id.to_owned(),
        resp: sendx,
    };

    sender.send(sync_op_msg).await?;
    recvx.await?
}

// TODO: move away frm memstore later
/// Creates a Blob ticket from the data we need to send
async fn fetch_encoded_delta_ticket(
    user_id: &str,
    sender: &tokio::sync::mpsc::Sender<SyncOp>,
    lrc: i64,
    store: &MemStore,
    addr: &EndpointAddr,
    content_key: &ContentKey,
) -> Result<BlobTicket> {
    let (sendx, recvx) = oneshot::channel();

    let sync_op_msg = SyncOp::GetEncodedData {
        user_id: user_id.to_owned(),
        last_row_counter: lrc,
        resp: sendx,
    };

    sender.send(sync_op_msg).await?;
    let encoded_data_result = recvx.await??;
    let encrypted_data = encrypt_by_content_key(&encoded_data_result, content_key)?;
    let tag = store
        .blobs()
        .add_bytes(Into::<Bytes>::into(encrypted_data))
        .await?;

    Ok(BlobTicket::new(addr.clone(), tag.hash, tag.format))
}

/// Process the `SyncStart` event. This is processed by the bg network listener
async fn on_sync_start_event(
    msg_sender: &GossipSender,
    sync_helpers: &mut SyncAcceptLoopData,
    msg: &NetworkMessage,
    senders: (
        tokio::sync::mpsc::Sender<SyncOp>,
        tokio::sync::mpsc::Sender<NetworkApiEvent>,
    ),
) -> Result<GossipLoopCode> {
    let (sync_db_channel_sender, _caller_sender) = senders;
    if let MessageBody::SyncStart {
        last_row_counter: lrc,
        auth_token: token,
        session_id,
        pub_key,
    } = &msg.body
    {
        if let Err(err) = verify_invocation_token(token, msg).await {
            let err_reason = format!("Sync request failed due to {}", err);
            return handle_sync_rejection_by_listener(
                &err_reason,
                &sync_helpers.user,
                msg,
                msg_sender,
                GossipLoopCode::Continue,
            )
            .await;
        }

        let sender_did = msg.from_did.clone();
        let content_key = ContentKey::new();
        let ticket = fetch_encoded_delta_ticket(
            &sync_helpers.user.user_id,
            &sync_db_channel_sender,
            lrc.context("lrc failed")?,
            &sync_helpers.store,
            &sync_helpers.endpoint.addr(),
            &content_key,
        )
        .await?;

        let session_keypair_tuple = create_kem_pair();
        sync_helpers
            .sync_sessions
            .insert(session_id.clone(), session_keypair_tuple.clone());

        let (encapped_key, enc_content_key) = encrypt_by_hpke(
            pub_key.clone(),
            HPKE_SYNC_DOMAIN_SEPARATION,
            content_key.to_bytes().clone(),
            session_id,
        )?;
        let receiver_last_row_counter =
            fetch_last_row_counter(&sender_did, &sync_db_channel_sender).await?;

        let common_conn = get_db_conn(&DBTYPE::COMMON)?;
        let invocation_token = create_auth_invocation_token(&msg.from_did, common_conn).await?;
        let delta_info = NetworkMessage::new(
            &sync_helpers.user,
            msg.is_online,
            MessageBody::SyncSendDeltaInfo {
                blob_ticket: ticket.to_string(),
                last_row_counter: Some(receiver_last_row_counter),
                auth_token: invocation_token,
                content_key: enc_content_key,
                encapped_key,
                pub_key: session_keypair_tuple.1,
                session_id: session_id.to_string(),
            },
            NetworkMessageType::Sync,
            msg.from_did.to_string(),
        );
        let delta_info = SignedNetworkMessage::sign(delta_info)?;
        msg_sender.broadcast(delta_info.to_bytes().into()).await?;
        info!("Sent blob ticket {} to {}", ticket, sender_did);
    }
    Ok(GossipLoopCode::Continue)
}

async fn handle_device_link_rejection_by_listener(
    err: &str,
    user: &User,
    msg: &NetworkMessage,
    network_sender: &GossipSender,
) -> Result<GossipLoopCode> {
    log::error!("{}", err);

    let link_reject_resp = NetworkMessage::new(
        user,
        msg.is_online,
        MessageBody::LinkRejected {
            reason: err.to_string(),
        },
        NetworkMessageType::Link,
        msg.from_did.clone(),
    );
    let link_reject_resp = SignedNetworkMessage::sign(link_reject_resp)?;
    network_sender
        .broadcast(link_reject_resp.to_bytes().into())
        .await?;

    Ok(GossipLoopCode::Continue)
}

async fn handle_sync_rejection_by_listener(
    err: &str,
    user: &User,
    msg: &NetworkMessage,
    msg_sender: &GossipSender,
    loop_code: GossipLoopCode,
) -> Result<GossipLoopCode> {
    log::error!("{}", err);
    let common_conn = get_db_conn(&DBTYPE::COMMON)?;
    let invocation_token = create_auth_invocation_token(&msg.from_did, common_conn).await?;
    let sync_reject_request = NetworkMessage::new(
        user,
        msg.is_online,
        MessageBody::SyncRejected {
            reason: err.to_string(),
            auth_token: invocation_token,
        },
        NetworkMessageType::Sync,
        msg.from_did.clone(),
    );
    let sync_reject_request = SignedNetworkMessage::sign(sync_reject_request)?;
    msg_sender
        .broadcast(sync_reject_request.to_bytes().into())
        .await?;

    Ok(loop_code)
}

/// Process `SyncSendDeltaInfo`. Processed by listeners and sender processes
async fn on_sync_send_delta_info(
    msg_sender: &GossipSender,
    sync_helpers: &mut SyncAcceptLoopData,
    msg: &NetworkMessage,
    senders: (
        tokio::sync::mpsc::Sender<SyncOp>,
        tokio::sync::mpsc::Sender<NetworkApiEvent>,
    ),
) -> Result<GossipLoopCode> {
    let (sync_db_channel_sender, _caller_sender) = senders;
    if let MessageBody::SyncSendDeltaInfo {
        blob_ticket,
        last_row_counter,
        auth_token,
        content_key,
        pub_key,
        encapped_key,
        session_id,
    } = &msg.body
    {
        if let Err(err) = verify_invocation_token(auth_token, msg).await {
            let err_reason = format!("Sync delta process failed due to {}", err);

            return handle_sync_rejection_by_listener(
                &err_reason,
                &sync_helpers.user,
                msg,
                msg_sender,
                GossipLoopCode::Continue,
            )
            .await;
        }
        let ticket: BlobTicket = blob_ticket.parse()?;
        let downloader = &sync_helpers.store.downloader(&sync_helpers.endpoint);
        downloader
            .download(ticket.hash(), Some(ticket.addr().id))
            .await?;

        let data = &sync_helpers.store.blobs().get_bytes(ticket.hash()).await?;
        info!("Downloaded data diff");

        if data.len() > MAX_DOWNLOADED_BYTES {
            let err_reason = format!(
                "Downloaded delta is greater than {}, skipping the sync",
                MAX_DOWNLOADED_BYTES
            );

            // last_row_counter None means listener received it from initiator
            let loop_code = if last_row_counter.is_none() {
                GossipLoopCode::Continue
            } else {
                GossipLoopCode::Exit
            };
            return handle_sync_rejection_by_listener(
                &err_reason,
                &sync_helpers.user,
                msg,
                msg_sender,
                loop_code,
            )
            .await;
        }

        let keypair = if let Some(keypair) = sync_helpers.sync_sessions.get(session_id.as_str()) {
            keypair
        } else {
            return Err(anyhow!("Failed to get session keypair"));
        };

        let content_key_bytes = decrypt_by_hpke(
            keypair.0.clone(),
            encapped_key.clone(),
            content_key.clone(),
            HPKE_SYNC_DOMAIN_SEPARATION,
            session_id,
        )?;

        let decrypted_delta =
            decrypt_by_content_key(data, &ContentKey::from_bytes(&content_key_bytes)?)?;
        let (sendx, recvx) = oneshot::channel();
        let sync_op_msg = SyncOp::ApplyDelta {
            delta: decrypted_delta,
            resp: sendx,
        };
        sync_db_channel_sender.send(sync_op_msg).await?;

        recvx.await??;
        info!("Diff applied successfully");
        let common_conn = get_db_conn(&DBTYPE::COMMON)?;
        let invocation_token = create_auth_invocation_token(&msg.from_did, common_conn).await?;
        // last_row_counter None means its end of sync relay
        if let Some(row_counter) = last_row_counter {
            // This block will be run by the initiator side
            let content_key = ContentKey::new();
            let ticket = fetch_encoded_delta_ticket(
                &sync_helpers.user.user_id,
                &sync_db_channel_sender,
                *row_counter,
                &sync_helpers.store,
                &sync_helpers.endpoint.addr(),
                &content_key,
            )
            .await?;

            let (encapped_key, enc_content_key) = encrypt_by_hpke(
                pub_key.clone(),
                HPKE_SYNC_DOMAIN_SEPARATION,
                content_key.to_bytes().clone(),
                session_id,
            )?;

            let delta_info = NetworkMessage::new(
                &sync_helpers.user,
                msg.is_online,
                MessageBody::SyncSendDeltaInfo {
                    blob_ticket: ticket.to_string(),
                    last_row_counter: None,
                    auth_token: invocation_token,
                    content_key: enc_content_key,
                    pub_key: keypair.1.clone(),
                    session_id: session_id.to_string(),
                    encapped_key,
                },
                NetworkMessageType::Sync,
                msg.from_did.clone(),
            );

            let delta_info = SignedNetworkMessage::sign(delta_info)?;
            msg_sender.broadcast(delta_info.to_bytes().into()).await?;
            info!("Sent blob ticket {} to {}", ticket, msg.from_did);
        } else {
            // remove the session data
            sync_helpers.sync_sessions.remove(session_id);
            let stop_req = NetworkMessage::new(
                &sync_helpers.user,
                msg.is_online,
                MessageBody::SyncEnd {
                    auth_token: invocation_token,
                    session_id: session_id.to_string(),
                },
                NetworkMessageType::Sync,
                msg.from_did.clone(),
            );
            let stop_req = SignedNetworkMessage::sign(stop_req)?;
            msg_sender.broadcast(stop_req.to_bytes().into()).await?;
            info!("sync ended");
            println!("\nSync completed....");
        }
    }
    Ok(GossipLoopCode::Continue)
}

pub async fn share(endpoint: Endpoint, mut recvx: oneshotReceiver<bool>) -> Result<()> {
    loop {
        tokio::select! {
                        _ = &mut recvx => {
                                info!("stopping the proxy, exiting");
                                endpoint.close().await;
                                break;
                        }
                        incoming = endpoint.accept() => {
                            let Some(incoming) = incoming else {
                                break;
                            };
                            let connection = incoming.await.map_err(|e| {
                                log::error!(
                                    "<remote_infy::host>::Error on connection incoming due to {:?}",e);
                                    e
                                })?;

                            tokio::spawn(async move {
                                loop {
                                    let (mut iroh_send, mut iroh_recv) = match connection.accept_bi().await {
                                        Ok(streams) => streams,
                                        Err(e) => {
                                            log::error!("<remote_infy::host> connection closed due to {:?}", e);
                                            break;
                                        }
                                    };

                                    tokio::spawn(async move {
                                        let mut local_tcp = TcpStream::connect("127.0.0.1:6969".to_string())
                                            .await
                                            .expect("Failed to connect to local service");

                                        let (mut local_read, mut local_write) = local_tcp.split();
                                        let local_to_remote = async {
                                            copy(&mut local_read, &mut iroh_send).await.map_err(|e| {
                                                log::error!(
                                                    "<remote_infy::host> failed forwarding local response to peer: {:?}",
                                                    e
                                                );
                                                e
                                            })?;
                                            iroh_send.finish().map_err(|e| {
                                                log::error!(
                                                    "<remote_infy::host> failed finishing response stream: {:?}",
                                                    e
                                                );
                                                e
                                            })?;
                                            Result::<()>::Ok(())
                                        };
                                        let remote_to_local = async {
                                            copy(&mut iroh_recv, &mut local_write).await.map_err(|e| {
                                                log::error!("<remote_infy::host> failed forwarding peer request to local server: {:?}", e);
                                                e
                                            })?;
                                            local_write.shutdown().await.map_err(|e| {
                                                log::error!("<remote_infy::host> failed shutting down local server write half: {:?}", e);
                                                e
                                            })?;
                                            Result::<()>::Ok(())
                                        };

                                        tokio::pin!(local_to_remote);
                                        tokio::pin!(remote_to_local);

                                        tokio::select! {
                                            result = &mut local_to_remote => {
                                                match result {
                                                    Ok(_) => (),
                                                    Err(e) => log::error!("<remote_infy::host> proxy failed: {:?}", e),
                                                }
                                            }
                                            result = &mut remote_to_local => {
                                                match result {
                                                    Ok(_) => {
                                                        match local_to_remote.await {
                                                            Ok(_) => (),
                                                            Err(e) => log::error!("<remote_infy::host> proxy failed: {:?}", e),
                                                        }
                                                    }
                                                    Err(e) => log::error!("<remote_infy::host> proxy failed: {:?}", e),
                                                }
                                            }
                                        }
                                    });
                                }
                            });

                        }
        }
    }
    info!("Endpoint closed, exiting");
    Ok(())
}

pub async fn connect(ticket: &str) -> Result<()> {
    let ticket: EndpointTicket = ticket.parse()?;
    let host_addr: EndpointAddr = ticket.into();

    let endpoint = Endpoint::bind(presets::N0).await?;

    let local_listener = TcpListener::bind("127.0.0.1:9271").await?;
    println!("Connected to remote inference");

    loop {
        let (mut local_tcp, _) = local_listener.accept().await.map_err(|e| {
            log::error!(
                "<reminf::peer>::Error on local listener accept due to {:?}",
                e
            );
            e
        })?;
        let endpoint = endpoint.clone();
        let host_addr = host_addr.clone();

        tokio::spawn(async move {
            let connection = endpoint.connect(host_addr, ALPN).await.unwrap();
            let (mut iroh_send, mut iroh_recv) = connection.open_bi().await.unwrap();
            let (mut local_read, mut local_write) = local_tcp.split();
            let local_to_remote = async {
                copy(&mut local_read, &mut iroh_send).await.map_err(|e| {
                    log::error!(
                        "<reminf::peer> failed forwarding local request to peer: {:?}",
                        e
                    );
                    e
                })?;
                iroh_send.finish().map_err(|e| {
                    log::error!("<reminf::peer> failed finishing request stream: {:?}", e);
                    e
                })?;
                Result::<()>::Ok(())
            };
            let remote_to_local = async {
                copy(&mut iroh_recv, &mut local_write).await.map_err(|e| {
                    log::error!(
                        "<reminf::peer> failed forwarding peer response to local client: {:?}",
                        e
                    );
                    e
                })?;
                local_write.shutdown().await.map_err(|e| {
                    log::error!(
                        "<reminf::peer> failed shutting down local client write half: {:?}",
                        e
                    );
                    e
                })?;
                Result::<()>::Ok(())
            };

            tokio::pin!(local_to_remote);
            tokio::pin!(remote_to_local);

            tokio::select! {
                result = &mut remote_to_local => {
                    match result {
                        Ok(_) => (),
                        Err(e) => log::error!("<reminf::peer> proxy failed: {:?}", e),
                    }
                }
                result = &mut local_to_remote => {
                    match result {
                        Ok(_) => {
                            match remote_to_local.await {
                                Ok(_) => (),
                                Err(e) => log::error!("<reminf::peer> proxy failed: {:?}", e),
                            }
                        }
                        Err(e) => log::error!("<reminf::peer> proxy failed: {:?}", e),
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use futures_util::StreamExt;
    use iroh::endpoint::presets;
    use iroh::protocol::Router;
    use iroh::{Endpoint, SecretKey, address_lookup::memory::MemoryLookup};
    use iroh_blobs::ticket::BlobTicket;
    use iroh_gossip::{Gossip, TopicId, api::Event};
    use tokio::sync::mpsc;

    use crate::core::{
        chats::SyncOp,
        network::{
            MessageBody, NetworkMessage, NetworkMessageType, ReplayCache, SignedNetworkMessage,
            fetch_last_row_counter, get_did_from_public_key,
        },
    };
    use crate::utils::crypto::{ContentKey, decrypt_by_content_key, encrypt_by_content_key};
    use crate::utils::get_unix_time_now;

    fn signed_test_message(secret_key: &SecretKey) -> SignedNetworkMessage {
        let from_did = get_did_from_public_key(secret_key.public().as_bytes()).unwrap();
        let message = NetworkMessage {
            from_did,
            from_nickname: "sender".to_string(),
            to_did: "did:key:test-recipient".to_string(),
            is_online: true,
            body: MessageBody::LinkAccepted,
            nonce: [7; 16],
            timestamp: get_unix_time_now(),
            msg_type: NetworkMessageType::Link,
        };

        SignedNetworkMessage::sign_with_secret_key(message, secret_key)
    }

    #[test]
    fn signed_network_message_verifies_with_signing_key() {
        let secret_key = SecretKey::generate();
        let signed_message = signed_test_message(&secret_key);

        signed_message
            .verify_with_public_key(&secret_key.public())
            .unwrap();
    }

    #[test]
    fn signed_network_message_rejects_tampered_message() {
        let secret_key = SecretKey::generate();
        let mut signed_message = signed_test_message(&secret_key);
        signed_message.msg.nonce[0] ^= 1;

        assert!(
            signed_message
                .verify_with_public_key(&secret_key.public())
                .is_err()
        );
    }

    #[test]
    fn signed_network_message_round_trips_through_wire_encoding() {
        let secret_key = SecretKey::generate();
        let signed_message = signed_test_message(&secret_key);
        let encoded = signed_message.to_bytes();
        let decoded = SignedNetworkMessage::from_bytes(&encoded).unwrap();

        decoded
            .verify_with_public_key(&secret_key.public())
            .unwrap();
    }

    #[test]
    fn sender_public_key_is_derived_from_signed_did() {
        let endpoint_key = SecretKey::generate();
        let signed_message = signed_test_message(&endpoint_key);

        assert_eq!(
            signed_message.sender_public_key().unwrap(),
            endpoint_key.public()
        );
    }

    #[test]
    fn replay_cache_rejects_duplicate_nonce_until_expiry() {
        let mut cache = ReplayCache::new();
        let nonce = [11; 16];

        assert!(cache.accept("did:key:sender", nonce, 100));
        assert!(!cache.accept("did:key:sender", nonce, 101));
        assert!(cache.accept(
            "did:key:sender",
            nonce,
            100 + super::MAX_MESSAGE_AGE_MILLI_SECS + 1
        ));
    }

    #[test]
    fn replay_cache_scopes_nonces_to_sender() {
        let mut cache = ReplayCache::new();
        let nonce = [12; 16];

        assert!(cache.accept("did:key:sender-a", nonce, 100));
        assert!(cache.accept("did:key:sender-b", nonce, 100));
    }

    #[tokio::test]
    async fn test_fetch_last_row_counter() {
        {
            let (tx, mut rx) = mpsc::channel::<SyncOp>(32);

            let _handler = tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let SyncOp::GetLastRowCounter { user_id: _, resp } = msg {
                        resp.send(Ok(1)).unwrap();
                    }
                }
            });
            assert_eq!(fetch_last_row_counter("did:key:xx", &tx).await.unwrap(), 1);
        }
    }

    #[tokio::test]
    async fn signed_link_request_is_delivered_over_gossip() {
        let sender_secret_key = SecretKey::generate();
        let sender_did = get_did_from_public_key(sender_secret_key.public().as_bytes()).unwrap();

        let sender_endpoint = Endpoint::builder(presets::Minimal)
            .secret_key(sender_secret_key.clone())
            .bind()
            .await
            .unwrap();
        let receiver_endpoint = Endpoint::bind(presets::Minimal).await.unwrap();

        let address_lookup = MemoryLookup::new();
        address_lookup.add_endpoint_info(sender_endpoint.addr());
        address_lookup.add_endpoint_info(receiver_endpoint.addr());
        sender_endpoint
            .address_lookup()
            .unwrap()
            .add(address_lookup.clone());
        receiver_endpoint
            .address_lookup()
            .unwrap()
            .add(address_lookup);

        let sender_gossip = Gossip::builder().spawn(sender_endpoint.clone());
        let receiver_gossip = Gossip::builder().spawn(receiver_endpoint.clone());
        let sender_router = Router::builder(sender_endpoint.clone())
            .accept(iroh_gossip::ALPN, sender_gossip.clone())
            .spawn();
        let receiver_router = Router::builder(receiver_endpoint.clone())
            .accept(iroh_gossip::ALPN, receiver_gossip.clone())
            .spawn();

        let topic_id = TopicId::from_bytes([42; 32]);
        let (sender_topic, receiver_topic) = tokio::try_join!(
            sender_gossip.subscribe_and_join(topic_id, vec![]),
            receiver_gossip.subscribe_and_join(topic_id, vec![sender_endpoint.id()]),
        )
        .unwrap();
        let (sender, _sender_receiver) = sender_topic.split();
        let (_receiver_sender, mut receiver) = receiver_topic.split();

        let message = NetworkMessage {
            from_did: sender_did,
            from_nickname: "sender".to_string(),
            to_did: "did:key:receiver".to_string(),
            is_online: true,
            body: MessageBody::LinkRequest {
                auth_token: "invocation-token".to_string(),
                peer_delegation_token: "delegation-token".to_string(),
                nickname: Some("sender".to_string()),
            },
            nonce: [9; 16],
            timestamp: get_unix_time_now(),
            msg_type: NetworkMessageType::Link,
        };
        let signed_message =
            SignedNetworkMessage::sign_with_secret_key(message, &sender_secret_key);
        sender
            .broadcast(signed_message.to_bytes().into())
            .await
            .unwrap();

        let received_message = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(Ok(Event::Received(message))) = receiver.next().await {
                    break message;
                }
            }
        })
        .await
        .expect("timed out waiting for gossip link request");

        let received_message = SignedNetworkMessage::from_bytes(&received_message.content).unwrap();
        received_message.verify().unwrap();
        assert!(matches!(
            received_message.msg.body,
            MessageBody::LinkRequest { .. }
        ));
        assert_eq!(
            received_message.msg.msg_type as u8,
            NetworkMessageType::Link as u8
        );

        sender_router.shutdown().await.unwrap();
        receiver_router.shutdown().await.unwrap();
        sender_endpoint.close().await;
        receiver_endpoint.close().await;
    }

    #[tokio::test]
    async fn blob_ticket_downloads_from_the_endpoint_that_stores_the_blob() {
        let provider_endpoint = Endpoint::bind(presets::Minimal).await.unwrap();
        let downloader_endpoint = Endpoint::bind(presets::Minimal).await.unwrap();

        let address_lookup = MemoryLookup::new();
        address_lookup.add_endpoint_info(provider_endpoint.addr());
        address_lookup.add_endpoint_info(downloader_endpoint.addr());
        provider_endpoint
            .address_lookup()
            .unwrap()
            .add(address_lookup.clone());
        downloader_endpoint
            .address_lookup()
            .unwrap()
            .add(address_lookup);

        let (_provider_gossip, provider_router, provider_store) =
            super::create_tiles_gossip_endpoint(&provider_endpoint)
                .await
                .unwrap();
        let (_downloader_gossip, downloader_router, downloader_store) =
            super::create_tiles_gossip_endpoint(&downloader_endpoint)
                .await
                .unwrap();

        let payload = b"sync delta payload".to_vec();
        let content_key = ContentKey::new();
        let encrypted_data = encrypt_by_content_key(&payload, &content_key).unwrap();
        let tag = provider_store
            .blobs()
            .add_bytes(encrypted_data.clone())
            .await
            .unwrap();
        let ticket = BlobTicket::new(provider_endpoint.addr(), tag.hash, tag.format);

        let downloader = downloader_store.downloader(&downloader_endpoint);
        downloader
            .download(ticket.hash(), Some(ticket.addr().id))
            .await
            .unwrap();

        let downloaded = downloader_store
            .blobs()
            .get_bytes(ticket.hash())
            .await
            .unwrap();
        let downloaded = decrypt_by_content_key(&downloaded, &content_key).unwrap();
        assert_eq!(downloaded, payload.as_slice());

        downloader_router.shutdown().await.unwrap();
        provider_router.shutdown().await.unwrap();
        downloader_endpoint.close().await;
        provider_endpoint.close().await;
    }
}

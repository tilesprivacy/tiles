//! The main module for networking

pub mod ticket;
use std::{str::FromStr, time::Duration};

use anyhow::{Result, anyhow};
use axum::body::Bytes;
use futures_util::{StreamExt, TryStreamExt};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, NET_REPORT_TIMEOUT, PublicKey,
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
        mpsc::{self, Receiver as MpscReceiver, Sender},
        oneshot::{self, Receiver as oneshotReceiver},
    },
    time::sleep,
};

use crate::{
    core::{
        account::{
            self, get_did_from_public_key, get_public_key_from_did, get_random_bytes,
            local::{
                Capabilities, Token, TokenType, User, add_token, create_invocation_token,
                create_token, fetch_delegated_tokens, get_app_secret_key, get_current_user,
                get_user_info, verify_invocation,
            },
        },
        chats::{SyncOp, create_db_channel},
        network::ticket::EndpointUserData,
        storage::db::{DBTYPE, get_db_conn},
    },
    daemon::net::{API_TIMEOUT, NetworkApiEvent},
};
use owo_colors::OwoColorize;
use sha2::{Digest, Sha256};

// 50 mb
const MAX_DOWNLOADED_BYTES: usize = 50 * 1024 * 1024;

const ALPN: &[u8] = b"remote-link/v1";

// const DEVICE_LINK_LOCAL_TOPIC: &str = "com.tilesprivacy.tiles.link";
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct NetworkMessage {
    from_did: String,
    from_nickname: String,
    is_online: bool,
    body: MessageBody,
    // to prevent iroh's deduplication on same msg
    nonce: [u8; 16],
    msg_type: NetworkMessageType,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum NetworkMessageType {
    Link,
    Sync,
}
impl NetworkMessage {
    fn new(
        user: &account::local::User,
        is_online: bool,
        body: MessageBody,
        msg_type: NetworkMessageType,
    ) -> Self {
        Self {
            from_did: user.user_id.clone(),
            from_nickname: user.username.clone(),
            is_online,
            body,
            nonce: get_random_bytes(),
            msg_type,
        }
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }
    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect("Failed to convert to bytes w postcard")
    }
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
        invocation_token: String,
        reverse_delegation_token: String,
        sender_nickname: String,
    },
    SyncSendDeltaInfo {
        blob_ticket: String,
        last_row_counter: Option<i64>,
    },
    SyncEnd,
    SyncRejected {
        reason: String,
    },
}

#[derive(Clone)]
pub struct SyncAcceptLoopData {
    user: User,
    store: MemStore,
    endpoint: Endpoint,
    db_channel_sender: Sender<SyncOp>,
}

/// Codes used inside the accept gossip loop that decided the exit or contination of the loop
enum GossipLoopCode {
    Exit,
    Continue,
}

enum DeviceLinkSelector {
    Error(String),
    Joined(GossipTopic),
}
//TODO: event signing to be done
// Current idea: Sign every event message with the private key
// Validate it on the other end
// In addition to this every request events must have ucan token authz
// with a header field like auth_token

/// Handles the iroh gossip events
async fn process_gossip_network_events(
    mut msg_receiver: GossipReceiver,
    msg_sender: GossipSender,
    user: account::local::User,
    sync_data: Option<SyncAcceptLoopData>,
    caller_sender: tokio::sync::mpsc::Sender<NetworkApiEvent>,
) -> Result<()> {
    while let Some(event) = msg_receiver.try_next().await? {
        info!(
            "Gossip Loop: Received by {}:, event {:?}",
            user.username, event
        );
        if let Event::Received(msg) = event {
            let pub_key = msg.delivered_from;
            let msg = NetworkMessage::from_bytes(&msg.content)?;
            // TODO: signature validation should happen here
            if !is_did_valid(&msg.from_did, pub_key)? {
                log::warn!(
                    "Incoming peer DID {} invalid, blocking request",
                    msg.from_did
                );
                continue;
            }
            let loop_code = match msg.msg_type {
                NetworkMessageType::Link => {
                    handle_device_link_events(msg, &user, &caller_sender, &msg_sender).await?
                }
                NetworkMessageType::Sync => {
                    if let Some(sync_datum) = sync_data.clone() {
                        handle_sync_events(msg, sync_datum, &caller_sender, pub_key, &msg_sender)
                            .await?
                    } else {
                        // revisit this, when doing sync
                        GossipLoopCode::Continue
                    }
                }
            };

            // exiting the accept loop, thus removing thyself from the topic on which this process was spawned
            if matches!(loop_code, GossipLoopCode::Exit) {
                info!("Exiting the gossip accepter loop");
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
    network_sender: &GossipSender,
) -> Result<GossipLoopCode> {
    log::info!("Handling device link requests");
    match msg.body {
        MessageBody::LinkRequest {
            auth_token,
            peer_delegation_token,
            nickname,
        } => {
            log::info!("on Link request");
            if let Err(err) = verify_invocation(&auth_token).await {
                let err_reason =
                    format!("link token invocation verification failed due to {:?}", err);
                return handle_device_link_rejection_by_listener(
                    &err_reason,
                    user,
                    msg.is_online,
                    network_sender,
                )
                .await;
            }

            let conn = get_db_conn(&DBTYPE::COMMON)?;

            if let Err(err) = add_token(&peer_delegation_token, &conn, nickname.as_deref()) {
                let err_reason = format!("adding peer delegation token failed due to {:?}", err);

                return handle_device_link_rejection_by_listener(
                    &err_reason,
                    user,
                    msg.is_online,
                    network_sender,
                )
                .await;
            }

            let link_resp = NetworkMessage::new(
                user,
                msg.is_online,
                MessageBody::LinkAccepted,
                NetworkMessageType::Link,
            );

            network_sender
                .broadcast(link_resp.to_bytes().into())
                .await?;
            // Adding a delay to prevent the risk of closing the endpoint before we send the msg via the above broadcast
            sleep(Duration::from_secs(5)).await;
        }
        MessageBody::LinkAccepted => {
            log::info!("On link accepted");
            caller_sender
                .send(NetworkApiEvent::DeviceLinkSuccess)
                .await?;

            return Ok(GossipLoopCode::Exit);
        }

        MessageBody::LinkRejected { reason } => {
            log::info!("On link rejected");
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
    sync_data: SyncAcceptLoopData,
    main_network_sender: &tokio::sync::mpsc::Sender<NetworkApiEvent>,
    pub_key: PublicKey,
    network_sender: &GossipSender,
) -> Result<GossipLoopCode> {
    match msg.body {
        MessageBody::SyncStart {
            last_row_counter: _,
            invocation_token: _,
            reverse_delegation_token: _,
            sender_nickname: _,
        } => {
            info!("Received sync start event...");
            let senders: (
                &tokio::sync::mpsc::Sender<SyncOp>,
                &tokio::sync::mpsc::Sender<NetworkApiEvent>,
            ) = (&sync_data.db_channel_sender, main_network_sender);
            on_sync_start_event(
                network_sender,
                &sync_data.store,
                &msg,
                pub_key,
                &sync_data.user,
                senders,
            )
            .await?;
        }
        MessageBody::SyncSendDeltaInfo {
            blob_ticket: _,
            last_row_counter: _,
        } => {
            let senders: (
                &tokio::sync::mpsc::Sender<SyncOp>,
                &tokio::sync::mpsc::Sender<NetworkApiEvent>,
            ) = (&sync_data.db_channel_sender, main_network_sender);
            on_sync_send_delta_info(
                network_sender,
                &sync_data.store,
                &msg,
                pub_key,
                &sync_data.user,
                &sync_data.endpoint,
                senders,
            )
            .await?;
        }
        MessageBody::SyncEnd => {
            println!("Sync completed..., exiting..");
            // TODO: Rethink if the event shld be stopnetwork or stop sync
            main_network_sender
                .send(NetworkApiEvent::StopNetwork)
                .await?;
        }
        MessageBody::SyncRejected { reason } => {
            println!(
                "Oops looks like your sync request has been rejected by {}({}),\nreason: {},\n Try again",
                msg.from_nickname, msg.from_did, reason
            );
            main_network_sender
                .send(NetworkApiEvent::StopNetwork)
                .await?;
        }
        msg_body => {
            info!("Invalid sync message {:?}", msg_body)
        }
    }
    Ok(GossipLoopCode::Continue)
}
/// Creats an Iroh endpoint. All the tiles communication references this endpoint
pub async fn create_endpoint(user: &account::local::User) -> Result<Endpoint> {
    info!("Creating endpoint for {}", user.user_id);
    // In release mode, we will build the endpoint using
    // tiles keypair in keychain
    let usr_data = EndpointUserData::new(&user.user_id, &user.username);
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

    let sync_data = SyncAcceptLoopData {
        user: user.clone(),
        store,
        endpoint: endpoint.clone(),
        db_channel_sender: db_channel_sender.clone(),
    };
    tokio::spawn(process_gossip_network_events(
        gossip_network_receiver,
        gossip_network_sender.clone(),
        user.clone(),
        Some(sync_data),
        main_network_sender.clone(),
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
    main_network_recv.recv().await;
    router_recv.shutdown().await?;
    endpoint.close().await;
    info!("Closing the tiles network listener");
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
            Some(&user.username),
            vec![Capabilities::All],
        )
        .await?
    };

    // create invocation token
    let invocation_token = create_invocation_token(link_token, conn).await?;

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

            tokio::spawn(process_gossip_network_events(
                gossip_network_receiver,
                gossip_network_sender.clone(),
                user.clone(),
                None,
                caller_sender.clone(),
            ));

            let link_req_msg = NetworkMessage::new(
                &user,
                true,
                MessageBody::LinkRequest {
                    auth_token: invocation_token,
                    peer_delegation_token: peer_delegation_token.token,
                    nickname: Some(user.username.clone()),
                },
                NetworkMessageType::Link,
            );
            gossip_network_sender
                .broadcast(link_req_msg.to_bytes().into())
                .await?;
            let r = caller_recv.recv().await;
            match r.unwrap() {
                NetworkApiEvent::DeviceLinkFailed(reason) => Err(anyhow!(reason)),
                _ => Ok(()),
            }
        }
    }
}
/// Entry point for the sync operation
///
/// if valid DID is passed, function will be in initiator mode
/// else will be in listening mode.
pub async fn sync(
    did: Option<String>,
    endpoint: Endpoint,
    main_network_sender: tokio::sync::mpsc::Sender<NetworkApiEvent>,
    sync_main_receiver: MpscReceiver<u8>,
    sync_api_resp_sender: tokio::sync::mpsc::Sender<NetworkApiEvent>,
) -> Result<()> {
    let user_db_conn = get_db_conn(&DBTYPE::COMMON)?;
    let user = get_current_user(&user_db_conn)?;
    let is_online = is_online(&endpoint).await;

    // handling the endpoint lookup separately for offline network using
    // mdns
    if !is_online {
        let mdns = MdnsAddressLookup::builder().build(endpoint.id())?;
        endpoint.address_lookup()?.add(mdns.clone());
    }

    // Channel to communicate from the `process_gossip_network_events`, which is a concurrent process, mainly to gracefully exit
    let mut sync_main_receiver = sync_main_receiver;
    // Creates a channel to communicate with the Database and pass the
    // sender across the tokio tasks
    let db_channel_sender = create_db_channel();

    if let Some(receiver_did) = did {
        // INITIATOR BLOCK
        // We only check if peer is already linked only for offline sync
        if let Err(_) = get_user_info(&user_db_conn, &receiver_did)
            && !is_online
        {
            eprintln!("The DID {} is not a linked peer", receiver_did);
            return Ok(());
        }

        let (invocation_token, rev_delegated_token) = if is_online {
            let tokens = fetch_delegated_tokens(
                &receiver_did,
                &user_db_conn,
                Some(account::local::TokenType::Linked),
            )?;
            let token_delegated = if tokens.len() == 1 {
                tokens[0].clone()
            } else {
                eprintln!("No sync authorization token found for {}", receiver_did);
                return Ok(());
            };
            let tokens = fetch_delegated_tokens(
                &user.user_id,
                &user_db_conn,
                Some(account::local::TokenType::Linked),
            )?;
            let aud_tokens: Vec<Token> = tokens
                .into_iter()
                .filter(|t| t.aud_did == receiver_did)
                .collect();

            let rev_delegated_token = if !aud_tokens.is_empty() {
                aud_tokens[0].token.clone()
            } else {
                info!("No reverse delegated token exist, creating one");
                // create a token
                create_token(&receiver_did, Some(&receiver_did), vec![Capabilities::All])
                    .await?
                    .token
            };

            let invocation_token =
                create_invocation_token(&token_delegated.token, user_db_conn).await?;
            (invocation_token, rev_delegated_token)
        } else {
            // We don't use invocation token in offline, so providing a dummy
            (String::from("offline token"), String::from("no token"))
        };

        let receiver_pub_key = get_public_key_from_did(&receiver_did)?;

        let receiver_endpoint_id = PublicKey::from_bytes(&receiver_pub_key)?;
        info!("receiver endpoint id {:?}", receiver_endpoint_id);

        // The sync gossip topic is basically derived from the receiver's
        // DID, so that initiator's can directly connect w/o any
        // initial handshake
        let sync_topic = format!("sync:{}", receiver_did);
        let sync_topic_id = create_topic_id(&sync_topic);

        let (network_sender, mut network_receiver, recv_router, store) =
            create_sync_network(&endpoint, sync_topic_id, vec![receiver_endpoint_id]).await?;

        println!("\nConnecting to {}.....", receiver_did);
        network_receiver.joined().await?;
        let sync_data = SyncAcceptLoopData {
            user: user.clone(),
            store,
            endpoint: endpoint.clone(),
            db_channel_sender: db_channel_sender.clone(),
        };
        tokio::spawn(process_gossip_network_events(
            network_receiver,
            network_sender.clone(),
            user.clone(),
            Some(sync_data),
            main_network_sender.clone(),
        ));

        let receiver_last_row_counter =
            fetch_last_row_counter(&receiver_did, &db_channel_sender).await?;
        let sync_start_msg = NetworkMessage::new(
            &user,
            is_online,
            MessageBody::SyncStart {
                last_row_counter: Some(receiver_last_row_counter),
                invocation_token,
                reverse_delegation_token: rev_delegated_token,
                sender_nickname: user.username.clone(),
            },
            NetworkMessageType::Sync,
        );
        network_sender
            .broadcast(sync_start_msg.to_bytes().into())
            .await?;
        info!("Sent SyncStart event");

        println!("\nSyncing in progress with ....{}", receiver_did);
        sync_main_receiver.recv().await;
        sync_api_resp_sender
            .send(NetworkApiEvent::DirectSyncDone)
            .await?;
        recv_router.shutdown().await?;
        endpoint.close().await;
    } else {
        // LISTENER BLOCK
        // The sync gossip topic is basically derived from the receiver's
        // public-key, so that initiator's can directly connect w/o any
        // initial handshake

        let did = if cfg!(debug_assertions) {
            let pub_key = endpoint.id();
            &get_did_from_public_key(pub_key.as_bytes())?
        } else {
            &user.user_id
        };

        let sync_topic = format!("sync:{}", did);
        let sync_topic_id = create_topic_id(&sync_topic);
        let (network_sender, network_receiver, recv_router, store) =
            create_sync_network(&endpoint, sync_topic_id, vec![]).await?;
        info!("sync gossip network created");
        let sync_data = SyncAcceptLoopData {
            user: user.clone(),
            store,
            endpoint: endpoint.clone(),
            db_channel_sender: db_channel_sender.clone(),
        };
        tokio::spawn(process_gossip_network_events(
            network_receiver,
            network_sender.clone(),
            user.clone(),
            Some(sync_data),
            main_network_sender.clone(),
        ));
        println!("{}", "Ready to accept sync requests from peers...".blue());
        sync_api_resp_sender
            .send(NetworkApiEvent::SyncListenerStarted)
            .await?;
        // Since in dev, we create endpoints randomly, at the initiator side
        // we can use the DID derived from this, instead of actual ones
        // for the network to form correctly
        if cfg!(debug_assertions) {
            println!("Use this DID {} in dev for testing", did);
        };
        sync_main_receiver.recv().await;
        recv_router.shutdown().await?;
        endpoint.close().await;
        // we probably can update the sync_endpoint_running variable here
        info!("Closing the sync listener")
    }
    Ok(())
}

/// Router with gossip and blob protocol
async fn create_sync_network(
    endpoint: &Endpoint,
    topic_id: TopicId,
    bootstrap_ids: Vec<iroh::PublicKey>,
) -> Result<(GossipSender, GossipReceiver, Router, MemStore)> {
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let store = MemStore::new();
    let blobs = BlobsProtocol::new(&store, None);
    let recv_router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .accept(iroh_blobs::ALPN, blobs.clone())
        .spawn();

    let (goss_sender, goss_receiver) = gossip.subscribe(topic_id, bootstrap_ids).await?.split();

    Ok((goss_sender, goss_receiver, recv_router, store))
}

/// Router with gossip and blob protocol
async fn create_tiles_gossip_endpoint(endpoint: &Endpoint) -> Result<(Gossip, Router, MemStore)> {
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

async fn is_online(endpoint: &Endpoint) -> bool {
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

fn is_did_valid(did: &str, pub_key: PublicKey) -> Result<bool> {
    // on debug mode, we skip the auth check, since we will be testing
    // with random endpoitns but w DID from config atp
    if cfg!(debug_assertions) {
        Ok(true)
    } else {
        Ok(get_did_from_public_key(&pub_key)? == did)
    }
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

async fn fetch_encoded_delta_ticket(
    user_id: &str,
    sender: &tokio::sync::mpsc::Sender<SyncOp>,
    lrc: i64,
    store: &MemStore,
    delivered_from: PublicKey,
) -> Result<BlobTicket> {
    let (sendx, recvx) = oneshot::channel();

    let sync_op_msg = SyncOp::GetEncodedData {
        user_id: user_id.to_owned(),
        last_row_counter: lrc,
        resp: sendx,
    };

    sender.send(sync_op_msg).await?;
    let encoded_data_result = recvx.await??;

    let tag = store
        .blobs()
        .add_bytes(Into::<Bytes>::into(encoded_data_result))
        .await?;

    Ok(BlobTicket::new(delivered_from.into(), tag.hash, tag.format))
}
async fn on_sync_start_event(
    network_sender: &GossipSender,
    store: &MemStore,
    msg: &NetworkMessage,
    delivered_from: PublicKey,
    user: &account::local::User,
    senders: (
        &tokio::sync::mpsc::Sender<SyncOp>,
        &tokio::sync::mpsc::Sender<NetworkApiEvent>,
    ),
) -> Result<()> {
    let (sync_db_channel_sender, main_network_sender) = senders;
    if let MessageBody::SyncStart {
        last_row_counter: lrc,
        invocation_token: token,
        reverse_delegation_token: rev_del_token,
        sender_nickname: nickname,
    } = &msg.body
    {
        if msg.is_online
            && let Err(err) = verify_invocation(token).await
        {
            log::warn!("Invocation verification failed due to {:?}", err);
            let reject_request = NetworkMessage::new(
                user,
                msg.is_online,
                MessageBody::SyncRejected {
                    reason: String::from("Sync failed due to invalid authorization"),
                },
                NetworkMessageType::Sync,
            );
            network_sender
                .broadcast(reject_request.to_bytes().into())
                .await?;
            // Adding a delay to prevent the risk of closing the endpoint before we send the msg via the above broadcast
            sleep(Duration::from_secs(5)).await;
            main_network_sender
                .send(NetworkApiEvent::StopNetwork)
                .await?;
            return Err(anyhow!("Verification failed for invocation token"));
        }
        if msg.is_online {
            //TODO: revist this, if we need a new conn here
            let conn = get_db_conn(&DBTYPE::COMMON)?;
            add_token(rev_del_token, &conn, Some(nickname))?;
        }

        let sender_did = get_did_from_public_key(delivered_from.as_bytes())?;
        let ticket = fetch_encoded_delta_ticket(
            &user.user_id,
            sync_db_channel_sender,
            lrc.expect("lrc failed"),
            store,
            delivered_from,
        )
        .await?;

        let receiver_last_row_counter =
            fetch_last_row_counter(&sender_did, sync_db_channel_sender).await?;

        let delta_info = NetworkMessage::new(
            user,
            msg.is_online,
            MessageBody::SyncSendDeltaInfo {
                blob_ticket: ticket.to_string(),
                last_row_counter: Some(receiver_last_row_counter),
            },
            NetworkMessageType::Sync,
        );
        network_sender
            .broadcast(delta_info.to_bytes().into())
            .await?;
        info!("Sent blob ticket {} to {}", ticket, sender_did);
    }
    Ok(())
}

async fn handle_device_link_rejection_by_listener(
    err: &str,
    user: &User,
    is_online: bool,
    network_sender: &GossipSender,
) -> Result<GossipLoopCode> {
    log::error!("{}", err);

    let link_reject_resp = NetworkMessage::new(
        user,
        is_online,
        MessageBody::LinkRejected {
            reason: err.to_string(),
        },
        NetworkMessageType::Link,
    );
    network_sender
        .broadcast(link_reject_resp.to_bytes().into())
        .await?;

    sleep(Duration::from_secs(5)).await;

    Ok(GossipLoopCode::Continue)
}

async fn on_sync_send_delta_info(
    network_sender: &GossipSender,
    store: &MemStore,
    msg: &NetworkMessage,
    delivered_from: PublicKey,
    user: &account::local::User,
    endpoint: &Endpoint,
    senders: (
        &tokio::sync::mpsc::Sender<SyncOp>,
        &tokio::sync::mpsc::Sender<NetworkApiEvent>,
    ),
) -> Result<()> {
    let (sync_db_channel_sender, _main_network_sender) = senders;
    if let MessageBody::SyncSendDeltaInfo {
        blob_ticket,
        last_row_counter,
    } = &msg.body
    {
        let ticket: BlobTicket = blob_ticket.parse()?;
        let downloader = store.downloader(endpoint);
        downloader
            .download(ticket.hash(), Some(delivered_from))
            .await?;

        let data = store.blobs().get_bytes(ticket.hash()).await?;
        info!("Downloaded data diff");

        if data.len() > MAX_DOWNLOADED_BYTES {
            log::error!(
                "Downloaded delta is greater than {}, skipping the sync",
                MAX_DOWNLOADED_BYTES
            );
            return Ok(());
        }

        let (sendx, recvx) = oneshot::channel();
        let sync_op_msg = SyncOp::ApplyDelta {
            delta: data.to_vec(),
            resp: sendx,
        };
        sync_db_channel_sender.send(sync_op_msg).await?;

        recvx.await??;
        info!("Diff applied successfully");

        // last_row_counter None means its end of sync relay
        if let Some(row_counter) = last_row_counter {
            let ticket = fetch_encoded_delta_ticket(
                &user.user_id,
                sync_db_channel_sender,
                *row_counter,
                store,
                delivered_from,
            )
            .await?;
            let delta_info = NetworkMessage::new(
                user,
                msg.is_online,
                MessageBody::SyncSendDeltaInfo {
                    blob_ticket: ticket.to_string(),
                    last_row_counter: None,
                },
                NetworkMessageType::Sync,
            );
            network_sender
                .broadcast(delta_info.to_bytes().into())
                .await?;
            info!("Sent blob ticket {} to {}", ticket, delivered_from);
        } else {
            let stop_req = NetworkMessage::new(
                user,
                msg.is_online,
                MessageBody::SyncEnd,
                NetworkMessageType::Sync,
            );
            network_sender.broadcast(stop_req.to_bytes().into()).await?;
            info!("sync ended");
            println!("\nSync completed....");
            // Adding a delay to prevent the risk of closing the endpoint before we send the msg via the above broadcast
            // We dont send the close event back, as the sync close is determined by listener
            // sleep(Duration::from_secs(5)).await;
            // main_network_sender.send(0).await?;
        }
    }
    Ok(())
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

    use tokio::sync::mpsc;

    use crate::core::{chats::SyncOp, network::fetch_last_row_counter};

    // #[tokio::test]
    // async fn test_valid_parse_link_ticket_online() {
    //     let topic_id = create_topic_id("test");
    //     let db_conn = setup_db_conn_v2();
    //     let user = create_dummy_user(&db_conn.common, None);
    //     let usr_data = EndpointUserData::new(&user.user_id, &user.username);
    //     let endpoint = Endpoint::builder(presets::N0)
    //         .user_data_for_address_lookup(UserData::try_from(usr_data.to_string()).unwrap())
    //         .bind()
    //         .await
    //         .unwrap();

    //     let ticket = LinkTicket::new(
    //         topic_id,
    //         endpoint.addr(),
    //         user.user_id.clone(),
    //         user.username.clone(),
    //     );

    //     assert!(parse_link_ticket(&ticket.to_string()).is_ok())
    // }

    // #[tokio::test]
    // async fn test_invalid_parse_link_ticket_online() {
    //     let topic_id = create_topic_id("test");
    //     let db_conn = setup_db_conn_v2();
    //     let user = create_dummy_user(&db_conn.common, None);
    //     let usr_data = EndpointUserData::new(&user.user_id, &user.username);
    //     let endpoint = Endpoint::builder(presets::N0)
    //         .user_data_for_address_lookup(UserData::try_from(usr_data.to_string()).unwrap())
    //         .bind()
    //         .await
    //         .unwrap();

    //     let ticket = LinkTicket::new(
    //         topic_id,
    //         endpoint.addr(),
    //         user.user_id.clone(),
    //         user.username.clone(),
    //     );

    //     let invalid_ticket = format!("{}xx", ticket);
    //     assert!(parse_link_ticket(&invalid_ticket).is_err())
    // }

    // #[test]
    // fn test_invalid_parse_link_ticket_offline() {
    //     let ticket = "kjadkjada";

    //     assert!(parse_link_ticket(ticket).is_err())
    // }

    // #[test]
    // fn test_valid_parse_link_ticket_offline() {
    //     let ticket = "kjadkja2";

    //     assert!(parse_link_ticket(ticket).is_ok())
    // }

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
}

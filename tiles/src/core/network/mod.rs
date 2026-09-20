//! The main module for networking

pub mod ticket;
use std::{
    io,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

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
    api::{Event, GossipReceiver, GossipSender},
};
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use iroh_tickets::endpoint::EndpointTicket;
use log::info;
use rusqlite::Connection;

use tokio::{
    io::{AsyncWriteExt, copy},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self},
        oneshot::{self, Receiver},
    },
    task::spawn_blocking,
    time::sleep,
};

use uuid::Uuid;

use crate::core::{
    account::{
        self, get_did_from_public_key, get_public_key_from_did, get_random_bytes,
        local::{
            add_token, create_invocation_token, create_token, fetch_token, fetch_token_by_aud,
            get_app_secret_key, get_current_user, get_user_info, save_peer_account_db,
            verify_invocation,
        },
    },
    chats::{SyncOp, create_db_sync_channel},
    network::ticket::EndpointUserData,
    storage::db::{DBTYPE, get_db_conn},
};
use owo_colors::OwoColorize;
use sha2::{Digest, Sha256};

// 50 mb
const MAX_DOWNLOADED_BYTES: usize = 50 * 1024 * 1024;

const ALPN: &[u8] = b"remote-link/v1";

const DEVICE_LINK_LOCAL_TOPIC: &str = "com.tilesprivacy.tiles.link";
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct NetworkMessage {
    from_did: String,
    from_nickname: String,
    is_online: bool,
    body: MessageBody,
    // to prevent iroh's deduplication on same msg
    nonce: [u8; 16],
}

impl NetworkMessage {
    fn new(user: &account::local::User, is_online: bool, body: MessageBody) -> Self {
        Self {
            from_did: user.user_id.clone(),
            from_nickname: user.username.clone(),
            is_online,
            body,
            nonce: get_random_bytes(),
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
        ticket: String,
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

pub async fn link(ticket: Option<String>) -> Result<()> {
    let user_db_conn = get_db_conn(&DBTYPE::COMMON)?;
    let user = get_current_user(&user_db_conn)?;
    let endpoint = create_endpoint(&user).await?;
    let is_online = is_online(&endpoint).await;
    let mut bootstrap_ids: Vec<EndpointId> = vec![];
    let (sendx, mut recvx) = mpsc::channel(1);
    // if ticket's there, then this is link enable sender's  command, else receiver end
    if let Some(ticket) = ticket {
        if is_online {
            println!(
                "You are online, but the given code is for offline. Please check your connection and retry the link process"
            );
            return Ok(());
        }
        let topic_id = create_topic_id(DEVICE_LINK_LOCAL_TOPIC);

        println!("Searching for peers in the local network..");
        let mdns = MdnsAddressLookup::builder().build(endpoint.id())?;
        let (new_bootstrap_ids, user_data) = find_offline_bootstrap_peers(&endpoint, mdns).await?;
        bootstrap_ids = new_bootstrap_ids;
        let endpoint_user_data = EndpointUserData::try_from(user_data.to_string())?;
        let did = endpoint_user_data.did;
        let nickname = endpoint_user_data.nickname;
        if get_user_info(&user_db_conn, &did).is_ok() {
            println!("Device {}({}) already linked", nickname, did);
            return Ok(());
        }
        let (sender, mut receiver, recv_router) =
            create_gossip_network(&endpoint, topic_id, bootstrap_ids).await?;

        println!("\nConnecting to {}({}).....", nickname, did);

        receiver.joined().await?;

        tokio::spawn(subsribe_loop(
            receiver,
            sender.clone(),
            user.clone(),
            user_db_conn,
            None,
            sendx.clone(),
        ));

        let link_req_msg =
            NetworkMessage::new(&user, is_online, MessageBody::LinkRequest { ticket });
        sender.broadcast(link_req_msg.to_bytes().into()).await?;

        println!("\nSent link request to {}({})", nickname, did);

        println!("\nWaiting for response...");

        recvx.recv().await;
        recv_router.shutdown().await?;
    } else {
        if is_online {
            println!(
                "You are online, so please provide peer DID with `tiles link create <PEER_DID>`"
            );
            return Ok(());
        }
        // RECEIVER BLOCK
        let mdns = MdnsAddressLookup::builder().build(endpoint.id())?;
        endpoint.address_lookup()?.add(mdns.clone());

        // Its better to have unique session'ed channels while
        // when the communication is over internet
        let topic_id = create_topic_id(DEVICE_LINK_LOCAL_TOPIC);

        let (sender, receiver, recv_router) =
            create_gossip_network(&endpoint, topic_id, bootstrap_ids).await?;

        let generated_ticket = {
            // generate a code
            let uuid = Uuid::new_v4().to_string();

            let ticket = uuid.split('-').collect::<Vec<&str>>()[0];

            println!("Generated link code: {}\n", ticket);

            println!(
                "Use this link code with `tiles link enable {}` on the system you want to connect to\n",
                ticket
            );
            ticket.to_string()
        };

        println!("Don't close this session until the link process is done\n");

        tokio::spawn(subsribe_loop(
            receiver,
            sender.clone(),
            user.clone(),
            user_db_conn,
            Some(generated_ticket),
            sendx.clone(),
        ));
        recvx.recv().await;
        recv_router.shutdown().await?;
    }
    endpoint.close().await;
    Ok(())
}

async fn subsribe_loop(
    mut receiver: GossipReceiver,
    sender: GossipSender,
    user: account::local::User,
    db_conn: Connection,
    generated_ticket: Option<String>,
    link_main_sender: tokio::sync::mpsc::Sender<u8>,
) -> Result<()> {
    while let Some(event) = receiver.try_next().await? {
        info!("from{}:", user.username);
        // TODO: Damn refactor the loop, its getting bigger
        if let Event::Received(msg) = event {
            let pub_key = msg.delivered_from;
            let msg = NetworkMessage::from_bytes(&msg.content)?;
            if !is_did_valid(&msg.from_did, pub_key)? {
                eprintln!(
                    "Incoming peer DID {} invalid, blocking request",
                    msg.from_did
                );
                continue;
            }
            match msg.body {
                MessageBody::LinkRequest { ticket } => {
                    println!(
                        "Received link request from {}({}), Do you want to link Y/N ?",
                        msg.from_nickname, msg.from_did
                    );
                    let input: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

                    let input_clone = input.clone();
                    let stdin = io::stdin();
                    spawn_blocking(move || {
                        let mut input_clone = input_clone.lock().unwrap();
                        let _ = stdin.read_line(&mut input_clone);
                    })
                    .await?;
                    let input_resp = input.lock().unwrap().trim().to_owned();

                    let link_res_resp = if input_resp.to_lowercase() == "y" {
                        if let Some(gen_ticket) = &generated_ticket
                            && !msg.is_online
                            && *gen_ticket != ticket.to_lowercase()
                        {
                            println!("\nVerifying code does not match, please try again");
                            let response = NetworkMessage::new(
                                &user,
                                msg.is_online,
                                MessageBody::LinkRejected {
                                    reason: String::from("Link code mismatch"),
                                },
                            );
                            sender.broadcast(response.to_bytes().into()).await?;
                            continue;
                        }

                        if let Err(err) =
                            save_peer_account_db(&db_conn, &msg.from_did, &msg.from_nickname)
                        {
                            println!("Failed to add the peer locally due to {:?}", err);

                            sleep(Duration::from_secs(5)).await;
                            link_main_sender.send(0).await?;
                        }

                        println!(
                            "Device {}({}) is now linked\n",
                            msg.from_nickname, msg.from_did
                        );
                        NetworkMessage::new(&user, msg.is_online, MessageBody::LinkAccepted)
                    } else {
                        NetworkMessage::new(
                            &user,
                            msg.is_online,
                            MessageBody::LinkRejected {
                                reason: String::from("Peer rejected the request"),
                            },
                        )
                    };
                    input.lock().unwrap().clear();

                    sender.broadcast(link_res_resp.to_bytes().into()).await?;
                    // Adding a delay to prevent the risk of closing the endpoint before we send the msg via the above broadcast
                    sleep(Duration::from_secs(5)).await;
                    link_main_sender.send(0).await?;
                }
                MessageBody::LinkAccepted => {
                    println!("\nLink accepted by {}({})", msg.from_nickname, msg.from_did);

                    if let Err(err) =
                        save_peer_account_db(&db_conn, &msg.from_did, &msg.from_nickname)
                    {
                        println!("Failed to add the peer locally due to {:?}", err);
                    }

                    link_main_sender.send(0).await?;
                }

                MessageBody::LinkRejected { reason } => {
                    println!(
                        "Oops looks like your link request has been rejected by {}({}),\nreason: {},\n Try again",
                        msg.from_nickname, msg.from_did, reason
                    );
                    link_main_sender.send(0).await?;
                }
                msg_body => {
                    eprintln!("Invalid link message {:?}", msg_body)
                }
            }
        }
    }
    Ok(())
}

/// Handles the iroh gossip eventd for the sync process
async fn sync_subscribe_loop(
    mut receiver: GossipReceiver,
    network_sender: GossipSender,
    user: account::local::User,
    store: MemStore,
    endpoint: Endpoint,
    sync_db_channel_sender: tokio::sync::mpsc::Sender<SyncOp>,
    sync_main_sender: tokio::sync::mpsc::Sender<u8>,
) -> Result<()> {
    while let Some(event) = receiver.try_next().await? {
        info!(
            "SYNC_LOOP: Received by {}:, event {:?}",
            user.username, event
        );
        if let Event::Received(msg) = event {
            let pub_key = msg.delivered_from;
            let msg = NetworkMessage::from_bytes(&msg.content)?;
            if !is_did_valid(&msg.from_did, pub_key)? {
                eprintln!(
                    "Incoming peer DID {} invalid, blocking request",
                    msg.from_did
                );
                continue;
            }
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
                        &tokio::sync::mpsc::Sender<u8>,
                    ) = (&sync_db_channel_sender, &sync_main_sender);
                    on_sync_start_event(&network_sender, &store, &msg, pub_key, &user, senders)
                        .await?;
                }
                MessageBody::SyncSendDeltaInfo {
                    blob_ticket: _,
                    last_row_counter: _,
                } => {
                    let senders: (
                        &tokio::sync::mpsc::Sender<SyncOp>,
                        &tokio::sync::mpsc::Sender<u8>,
                    ) = (&sync_db_channel_sender, &sync_main_sender);
                    on_sync_send_delta_info(
                        &network_sender,
                        &store,
                        &msg,
                        pub_key,
                        &user,
                        &endpoint,
                        senders,
                    )
                    .await?;
                }
                MessageBody::SyncEnd => {
                    println!("Sync completed..., exiting..");
                    sync_main_sender.send(0).await?;
                }
                MessageBody::SyncRejected { reason } => {
                    println!(
                        "Oops looks like your sync request has been rejected by {}({}),\nreason: {},\n Try again",
                        msg.from_nickname, msg.from_did, reason
                    );
                    sync_main_sender.send(0).await?;
                }
                msg_body => {
                    info!("Invalid sync message {:?}", msg_body)
                }
            }
        }
    }
    Ok(())
}
pub async fn create_endpoint(user: &account::local::User) -> Result<Endpoint> {
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

/// Entry point for the sync operation
///
/// if valid DID is passed, function will be in initiator mode
/// else will be in listening mode.
pub async fn sync(did: Option<String>) -> Result<()> {
    let user_db_conn = get_db_conn(&DBTYPE::COMMON)?;
    let user = get_current_user(&user_db_conn)?;
    let endpoint = create_endpoint(&user).await?;
    let is_online = is_online(&endpoint).await;

    // handling the endpoint lookup separately for offline network using
    // mdns
    if !is_online {
        let mdns = MdnsAddressLookup::builder().build(endpoint.id())?;
        endpoint.address_lookup()?.add(mdns.clone());
    }

    // Channel to communicate from the `sync_subscribe_loop`, which is a concurrent process, mainly to gracefully exit
    let (sync_main_sender, mut sync_main_receiver) = mpsc::channel(1);

    // Creates a channel to communicate with the Database and pass the
    // sender across the tokio tasks
    let db_channel_sender = create_db_sync_channel();

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
            let token_delegated = if let Ok(token_resp) = fetch_token(
                &receiver_did,
                &user_db_conn,
                account::local::TokenType::Sync,
            ) && let Some(token) = token_resp
            {
                // fetch token delegated to receiver, if not create one

                token
            } else {
                eprintln!("No sync authorization token found for {}", receiver_did);
                return Ok(());
            };
            let rev_token_resp = fetch_token_by_aud(
                &receiver_did,
                &user_db_conn,
                account::local::TokenType::Sync,
            )?;
            let rev_delegated_token = if let Some(rev_token) = rev_token_resp {
                rev_token.token
            } else {
                // create a token
                create_token(
                    &receiver_did,
                    Some(&receiver_did),
                    account::local::TokenType::Sync,
                )
                .await?
            };
            (
                create_invocation_token(&token_delegated.token, &user_db_conn).await?,
                rev_delegated_token,
            )
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
        tokio::spawn(sync_subscribe_loop(
            network_receiver,
            network_sender.clone(),
            user.clone(),
            store,
            endpoint.clone(),
            db_channel_sender.clone(),
            sync_main_sender.clone(),
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
        );
        network_sender
            .broadcast(sync_start_msg.to_bytes().into())
            .await?;
        info!("Sent SyncStart event");

        println!("\nSyncing in progress with ....{}", receiver_did);
        sync_main_receiver.recv().await;
        recv_router.shutdown().await?;
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
        tokio::spawn(sync_subscribe_loop(
            network_receiver,
            network_sender.clone(),
            user.clone(),
            store,
            endpoint.clone(),
            db_channel_sender.clone(),
            sync_main_sender.clone(),
        ));
        println!("{}", "Ready to accept sync requests from peers...".blue());

        // Since in dev, we create endpoints randomly, at the initiator side
        // we can use the DID derived from this, instead of actual ones
        // for the network to form correctly
        if cfg!(debug_assertions) {
            println!("Use this DID {} in dev for testing", did);
        };
        sync_main_receiver.recv().await;
        recv_router.shutdown().await?;
    }
    endpoint.close().await;
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
async fn find_offline_bootstrap_peers(
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

async fn create_gossip_network(
    endpoint: &Endpoint,
    topic_id: TopicId,
    bootstrap_ids: Vec<iroh::PublicKey>,
) -> Result<(GossipSender, GossipReceiver, Router)> {
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let recv_router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    let (goss_sender, goss_receiver) = gossip.subscribe(topic_id, bootstrap_ids).await?.split();

    Ok((goss_sender, goss_receiver, recv_router))
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
        &tokio::sync::mpsc::Sender<u8>,
    ),
) -> Result<()> {
    let (sync_db_channel_sender, sync_main_sender) = senders;
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
            );
            network_sender
                .broadcast(reject_request.to_bytes().into())
                .await?;
            // Adding a delay to prevent the risk of closing the endpoint before we send the msg via the above broadcast
            sleep(Duration::from_secs(5)).await;
            sync_main_sender.send(0).await?;
            return Err(anyhow!("Verification failed for invocation token"));
        }
        if msg.is_online {
            //TODO: revist this, if we need a new conn here
            let conn = get_db_conn(&DBTYPE::COMMON)?;
            add_token(
                rev_del_token,
                &conn,
                Some(&nickname),
                account::local::TokenType::Sync,
            )?;
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
        );
        network_sender
            .broadcast(delta_info.to_bytes().into())
            .await?;
        info!("Sent blob ticket {} to {}", ticket, sender_did);
    }
    Ok(())
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
        &tokio::sync::mpsc::Sender<u8>,
    ),
) -> Result<()> {
    let (sync_db_channel_sender, sync_main_sender) = senders;
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
            );
            network_sender
                .broadcast(delta_info.to_bytes().into())
                .await?;
            info!("Sent blob ticket {} to {}", ticket, delivered_from);
        } else {
            let stop_req = NetworkMessage::new(user, msg.is_online, MessageBody::SyncEnd);
            network_sender.broadcast(stop_req.to_bytes().into()).await?;
            info!("sync ended");
            println!("\nSync completed..., exiting now..");
            // Adding a delay to prevent the risk of closing the endpoint before we send the msg via the above broadcast
            sleep(Duration::from_secs(5)).await;
            sync_main_sender.send(0).await?;
        }
    }
    Ok(())
}

pub async fn share(endpoint: Endpoint, mut recvx: Receiver<bool>) -> Result<()> {
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

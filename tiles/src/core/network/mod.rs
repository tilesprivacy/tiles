//! The main module for networking

pub mod ticket;
use std::{
    io,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use axum::body::Bytes;
use futures_util::{StreamExt, TryStreamExt};
use iroh::{
    Endpoint, EndpointId, NET_REPORT_TIMEOUT, PublicKey,
    address_lookup::{self, MdnsAddressLookup, mdns},
    endpoint::{BindError, presets},
    endpoint_info::UserData,
    protocol::Router,
};
use iroh_blobs::{BlobsProtocol, store::mem::MemStore, ticket::BlobTicket};
use iroh_gossip::{
    Gossip, TopicId,
    api::{Event, GossipReceiver, GossipSender},
};

use log::info;
use rusqlite::Connection;
use tilekit::accounts::{
    get_did_from_public_key, get_public_key_from_did, get_random_bytes, get_random_bytes_32,
};
use tokio::{
    sync::{
        mpsc::{self},
        oneshot::{self},
    },
    task::spawn_blocking,
    time::sleep,
};
use uuid::Uuid;

use crate::core::{
    account::{
        self,
        local::{
            create_dummy_user, get_app_secret_key, get_current_user, get_user_info,
            save_peer_account_db,
        },
    },
    chats::{SyncOp, create_sync_channel},
    network::ticket::{EndpointUserData, LinkTicket},
    storage::db::{DBTYPE, get_db_conn},
};
use owo_colors::OwoColorize;
use sha2::{Digest, Sha256};

// 50 mb
const MAX_DOWNLOADED_BYTES: usize = 50 * 1024 * 1024;

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
    },
    SyncSendDeltaInfo {
        blob_ticket: String,
        last_row_counter: Option<i64>,
    },
    SyncEnd,
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
        let (endpoint_id, mut did, mut nickname, topic_value) = parse_link_ticket(&ticket)?;

        let topic_id = if is_online {
            topic_value.expect("Expected topicId")
        } else {
            create_topic_id(DEVICE_LINK_LOCAL_TOPIC)
        };

        if is_online {
            bootstrap_ids.push(endpoint_id.expect("Expected an EndpointId as bootstrapId "))
        } else {
            println!("Searching for peers in the local network..");
            let mdns = address_lookup::mdns::MdnsAddressLookup::builder().build(endpoint.id())?;
            let (new_bootstrap_ids, user_data) =
                find_offline_bootstrap_peers(&endpoint, mdns).await?;
            bootstrap_ids = new_bootstrap_ids;
            let endpoint_user_data = EndpointUserData::try_from(user_data.to_string())?;
            did = endpoint_user_data.did;
            nickname = endpoint_user_data.nickname;
        };
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
        // RECEIVER BLOCK
        if !is_online {
            let mdns = address_lookup::mdns::MdnsAddressLookup::builder().build(endpoint.id())?;
            endpoint.address_lookup()?.add(mdns.clone());
        }

        // Its better to have unique session'ed channels while
        // when the communication is over internet
        let topic_id = if is_online {
            TopicId::from_bytes(get_random_bytes_32())
        } else {
            create_topic_id(DEVICE_LINK_LOCAL_TOPIC)
        };

        let (sender, receiver, recv_router) =
            create_gossip_network(&endpoint, topic_id, bootstrap_ids).await?;

        let generated_ticket = if is_online {
            let ticket = LinkTicket::new(
                topic_id,
                endpoint.addr(),
                user.user_id.clone(),
                user.username.clone(),
            );
            println!("Generated link ticket: \n{:?}\n", ticket.to_string());

            println!(
                "Use this ticket with `tiles link enable <ticket>` on the system you want to connect to\n"
            );
            ticket.to_string()
        } else {
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
                    // Adding a delay to prevent the risk od closing the endpoint
                    // before we send the msg
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
                    sleep(Duration::from_secs(5)).await;
                    link_main_sender.send(0).await?;
                }

                MessageBody::LinkRejected { reason } => {
                    println!(
                        "Oops looks like your link request has been rejected by {}({}),\nreason: {},\n Try again",
                        msg.from_nickname, msg.from_did, reason
                    );
                    sleep(Duration::from_secs(5)).await;
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

async fn sync_subscribe_loop(
    mut receiver: GossipReceiver,
    sender: GossipSender,
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
                } => {
                    info!("Received sync start event...");
                    on_sync_start_event(
                        &sender,
                        &store,
                        &msg,
                        pub_key,
                        &user,
                        &sync_db_channel_sender,
                    )
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
                        &sender, &store, &msg, pub_key, &user, &endpoint, senders,
                    )
                    .await?;
                }
                MessageBody::SyncEnd => {
                    println!("Sync completed..., exiting..");
                    sleep(Duration::from_secs(5)).await;
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
async fn create_endpoint(user: &account::local::User) -> Result<Endpoint> {
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

pub async fn sync(did: Option<String>) -> Result<()> {
    let user_db_conn = get_db_conn(&DBTYPE::COMMON)?;
    let user = get_current_user(&user_db_conn)?;
    let endpoint = create_endpoint(&user).await?;
    let is_online = is_online(&endpoint).await;
    if !is_online {
        let mdns = address_lookup::mdns::MdnsAddressLookup::builder().build(endpoint.id())?;
        endpoint.address_lookup()?.add(mdns.clone());
    }
    let (sendx, mut recvx) = mpsc::channel(1);
    let tx = create_sync_channel();
    if let Some(receiver_did) = did {
        // INITIATOR BLOCK
        // The sync gossip topic is basically derived from the receiver's
        // DID, so that initiator's can directly connect w/o any
        // initial handshake
        let receiver_pub_key = get_public_key_from_did(&receiver_did)?;
        let receiver_user = if let Ok(receiver_user) = get_user_info(&user_db_conn, &receiver_did) {
            receiver_user
        } else {
            if cfg!(debug_assertions) == false {
                eprintln!("The DID {} is not a linked peer", receiver_did);
                return Ok(());
            }
            info!("creating a dummy user");
            create_dummy_user()
        };

        let receiver_endpoint_id = PublicKey::from_bytes(&receiver_pub_key)?;
        info!("receiver endpoint id {:?}", receiver_endpoint_id);
        let sync_topic = format!("sync:{}", receiver_did);
        let sync_topic_id = create_topic_id(&sync_topic);

        let (sender, mut receiver, recv_router, store) =
            create_sync_network(&endpoint, sync_topic_id, vec![receiver_endpoint_id]).await?;
        println!("\nConnecting to {}.....", receiver_did);
        receiver.joined().await?;
        tokio::spawn(sync_subscribe_loop(
            receiver,
            sender.clone(),
            user.clone(),
            store,
            endpoint.clone(),
            tx.clone(),
            sendx.clone(),
        ));

        let receiver_last_row_counter = fetch_last_row_counter(&receiver_did, &tx).await?;
        let sync_start_msg = NetworkMessage::new(
            &user,
            is_online,
            MessageBody::SyncStart {
                last_row_counter: Some(receiver_last_row_counter),
            },
        );
        sender.broadcast(sync_start_msg.to_bytes().into()).await?;
        info!("Sent SyncStart event");

        println!(
            "\nSyncing in progress with ....{}({})",
            receiver_user.username, receiver_did
        );
        recvx.recv().await;
        recv_router.shutdown().await?;
    } else {
        // RECEIVER BLOCK
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
        let (sender, receiver, recv_router, store) =
            create_sync_network(&endpoint, sync_topic_id, vec![]).await?;
        info!("sync gossip network created");
        tokio::spawn(sync_subscribe_loop(
            receiver,
            sender.clone(),
            user.clone(),
            store,
            endpoint.clone(),
            tx.clone(),
            sendx.clone(),
        ));
        println!("{}", "Ready to accept sync requests from peers...".blue());

        // Since in dev, we create endpoints randomly, at the initiator side
        // we can use the DID derived from this, instead of actual ones
        // for the network to form correctly
        if cfg!(debug_assertions) {
            println!("Use this DID {} in dev for testing", did);
        };
        recvx.recv().await;
        recv_router.shutdown().await?;
    }
    endpoint.close().await;
    Ok(())
}

// Router with gossip and blob protocol
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
            mdns::DiscoveryEvent::Discovered {
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
            mdns::DiscoveryEvent::Expired { endpoint_id } => {
                if cfg!(debug_assertions) {
                    println!("peer left {:?}", endpoint_id)
                }
            }
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

// We handle the parsing in this way since ticket can be an encoded `LinkTicket`
// or just a 4 byte hex if linking over mDNS
fn parse_link_ticket(
    ticket: &str,
) -> Result<(Option<EndpointId>, String, String, Option<TopicId>)> {
    if let Ok(parsed_ticket) = LinkTicket::from_str(ticket) {
        Ok((
            Some(parsed_ticket.addr.id),
            parsed_ticket.did,
            parsed_ticket.nickname,
            Some(parsed_ticket.topic_id),
        ))
    } else if ticket.len() == 8 {
        // NOTE: We only have len check as a "parser" for the offline code
        // but this will surely change once we fix the code format
        Ok((None, String::from(""), String::from(""), None))
    } else {
        Err(anyhow::anyhow!("Invalid Ticket"))
    }
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
    sender: &GossipSender,
    store: &MemStore,
    msg: &NetworkMessage,
    delivered_from: PublicKey,
    user: &account::local::User,
    sync_db_channel_sender: &tokio::sync::mpsc::Sender<SyncOp>,
) -> Result<()> {
    if let MessageBody::SyncStart {
        last_row_counter: lrc,
    } = &msg.body
    {
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
        sender.broadcast(delta_info.to_bytes().into()).await?;
        info!("Sent blob ticket {} to {}", ticket, sender_did);
    }
    Ok(())
}

async fn on_sync_send_delta_info(
    sender: &GossipSender,
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
            sender.broadcast(delta_info.to_bytes().into()).await?;
            info!("Sent blob ticket {} to {}", ticket, delivered_from);
        } else {
            let stop_req = NetworkMessage::new(user, msg.is_online, MessageBody::SyncEnd);
            sender.broadcast(stop_req.to_bytes().into()).await?;
            info!("sync ended");
            println!("\nSync completed..., exiting now..");
            sleep(Duration::from_secs(5)).await;
            sync_main_sender.send(0).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use iroh::{Endpoint, endpoint::presets, endpoint_info::UserData};
    use tokio::sync::mpsc;

    use crate::core::{
        account::local::create_dummy_user,
        chats::SyncOp,
        network::{
            create_topic_id, fetch_last_row_counter, parse_link_ticket,
            ticket::{EndpointUserData, LinkTicket},
        },
    };

    #[tokio::test]
    async fn test_valid_parse_link_ticket_online() {
        let topic_id = create_topic_id("test");
        let user = create_dummy_user();
        let usr_data = EndpointUserData::new(&user.user_id, &user.username);
        let endpoint = Endpoint::builder(presets::N0)
            .user_data_for_address_lookup(UserData::try_from(usr_data.to_string()).unwrap())
            .bind()
            .await
            .unwrap();

        let ticket = LinkTicket::new(
            topic_id,
            endpoint.addr(),
            user.user_id.clone(),
            user.username.clone(),
        );

        assert!(parse_link_ticket(&ticket.to_string()).is_ok())
    }

    #[tokio::test]
    async fn test_invalid_parse_link_ticket_online() {
        let topic_id = create_topic_id("test");
        let user = create_dummy_user();
        let usr_data = EndpointUserData::new(&user.user_id, &user.username);
        let endpoint = Endpoint::builder(presets::N0)
            .user_data_for_address_lookup(UserData::try_from(usr_data.to_string()).unwrap())
            .bind()
            .await
            .unwrap();

        let ticket = LinkTicket::new(
            topic_id,
            endpoint.addr(),
            user.user_id.clone(),
            user.username.clone(),
        );

        let invalid_ticket = format!("{}xx", ticket);
        assert!(parse_link_ticket(&invalid_ticket).is_err())
    }

    #[test]
    fn test_invalid_parse_link_ticket_offline() {
        let ticket = "kjadkjada";

        assert!(parse_link_ticket(ticket).is_err())
    }

    #[test]
    fn test_valid_parse_link_ticket_offline() {
        let ticket = "kjadkja2";

        assert!(parse_link_ticket(ticket).is_ok())
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
}

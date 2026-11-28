//! Module for handling networking for remote inference

use anyhow::{Result, anyhow};
use axum::response::sse::Event;
use iroh::{
    Endpoint,
    endpoint::{Connection, RecvStream, SendStream},
    protocol::{AcceptError, ProtocolHandler},
};

use iroh_tickets::endpoint::EndpointTicket;
use log::info;
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy},
    net::{
        TcpListener, TcpStream,
        tcp::{ReadHalf, WriteHalf},
    },
    sync::{mpsc::Sender, oneshot::Receiver as oneshotReceiver},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::atproto::event;

const REMOTE_INFERENCE_ALPN: &[u8] = b"remote-inference/v2";

#[derive(Debug, Clone)]
pub struct RemoteInference;

impl RemoteInference {
    pub fn get_alpn() -> &'static [u8] {
        REMOTE_INFERENCE_ALPN
    }
}

const TCP_PROXY_PORT: u32 = 9271;

impl ProtocolHandler for RemoteInference {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), AcceptError> {
        let endpoint_id = connection.remote_id();
        info!("Accepted connection from endpoint {:?}", endpoint_id);

        //TODO: How are we gonna stop this loop
        loop {
            let (mut iroh_send, mut iroh_recv) = connection.accept_bi().await?;
            tokio::spawn(async move {
                // TODO: use the PY_PORT here..
                let mut local_tcp = TcpStream::connect("127.0.0.1:6969".to_string())
                    .await
                    .expect("Failed to connect to local service");

                let (mut local_read, mut local_write) = local_tcp.split();

                // let local_to_remote = send_tcp_bytes(&mut local_read, &mut iroh_send);
                // let remote_to_local = accept_quic_bytes(&mut local_write, &mut iroh_recv);

                // tokio::pin!(local_to_remote);
                // tokio::pin!(remote_to_local);

                // tokio::select! {
                //     result = &mut local_to_remote=> {
                //         match result {
                //             Ok(_) => (),
                //             Err(e) => {
                //                 log::error!("<remote_infy::host> proxy failed: {:?}", e)
                //             }
                //         }
                //     }
                //     result = &mut remote_to_local => {
                //         match result {
                //             Ok(_) => {
                //                 match local_to_remote.await {
                //                     Ok(_) => (),
                //                     Err(e) => {
                //                         log::error!("<remote_infy::host> proxy failed: {:?}", e)
                //                     }
                //                 }
                //             }
                //             Err(e) => {
                //                 log::error!("<remote_infy::host> proxy failed: {:?}", e)
                //             }
                //         }
                //     }
                // }
            });
        }
    }
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
                            let connection  = match incoming.await {
                                Ok(conn) => conn,
                                Err(err) => {
                                    log::error!("<remote_infy::host>::Error on connection incoming due to {:?}",err);
                                continue;
                                }
                            };


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

/// Connects to the endpoint represented by the token, then send and receive bytes from the remote inference server.
pub async fn connect(
    token: String,
    endpoint: Endpoint,
    disconnect_token: CancellationToken,
    event_sender: Sender<Event>,
) -> Result<()> {
    // TODO: Will add support for UCAN tokens token in nxt phase
    let ticket: EndpointTicket = token.parse()?;

    let addr = format!("127.0.0.1:{}", TCP_PROXY_PORT);

    // Open a TCP connection locally
    let local_tcp_listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            handle_inference_error(
                &event_sender,
                "Failed to bind at TCP port",
                &err.to_string(),
            )
            .await;
            anyhow::bail!(err);
        }
    };

    // Establish connection with peer
    let connection = match endpoint
        .connect(ticket.clone(), REMOTE_INFERENCE_ALPN)
        .await
    {
        Ok(conn) => conn,
        Err(err) => {
            let err_reason = format!(
                "Failed to establish a connection with the peer {}",
                ticket.endpoint_addr().id
            );
            handle_inference_error(&event_sender, &err_reason, &err.to_string()).await;
            anyhow::bail!(err);
        }
    };

    event_sender.send(event("connected", json!(""))).await?;

    info!("Connected to remote inference");

    // disconnect_token.run_until_cancelled(fut)
    loop {
        tokio::select! {
            err = connection.closed() => {
                    disconnect_token.cancel();
                    handle_inference_error(
                        &event_sender,
                        "Peer disconnected",
                        &err.to_string(),
                        )
                        .await;
                        anyhow::bail!(err);
            }
            _ = disconnect_token.cancelled() => break,
            // For every TCP request, we open a quic bi-directional stream
            accept_tcp_result = local_tcp_listener.accept() => {
                let conn_id =  Uuid::now_v7().to_string();
                info!("TCP request with id {}", conn_id);
                let (local_tcp, _) = match accept_tcp_result {
                    Ok((stream, socket)) => (stream, socket),
                    Err(err) => {
                        handle_inference_error(
                            &event_sender,
                            "Error on accepting a new TCP connection (client)",
                            &err.to_string(),
                        )
                        .await;
                        anyhow::bail!(err);
                    }
                };
                let conn = connection.clone();
                let event_senderx = event_sender.clone();
                let disconnect_token_clone = disconnect_token.clone();
                let conn_id_clone = conn_id.clone();
                tokio::spawn(disconnect_token_clone.run_until_cancelled_owned(process_tcp_requests(conn, event_senderx, local_tcp, conn_id_clone)));
                continue;

            }
        };
    }
    event_sender
        .send(event("DONE", json!("")))
        .await
        .map_err(Into::into)
}

async fn process_tcp_requests(
    conn: Connection,
    event_sender: Sender<Event>,
    local_tcp: TcpStream,
    session_id: String,
) -> Result<((), ())> {
    let mut local_tcp = local_tcp;
    let (mut iroh_send, mut iroh_recv) = match conn.open_bi().await {
        Ok((send, recv)) => (send, recv),
        Err(err) => {
            handle_inference_error(
                &event_sender,
                "Failed to initiate a bi-directional stream",
                &err.to_string(),
            )
            .await;
            anyhow::bail!(err);
        }
    };
    info!("TCP session {session_id}: QUIC stream opened");
    let (mut local_read, mut local_write) = local_tcp.split();
    let local_to_remote =
        send_local_request(&mut local_read, &mut iroh_send, &event_sender, &session_id);
    let remote_to_local =
        accept_remote_response(&mut local_write, &mut iroh_recv, &event_sender, &session_id);

    tokio::pin!(local_to_remote, remote_to_local);
    tokio::select! {
        biased;

        response = &mut remote_to_local => {
            response?;
            Ok(((), ()))
        }
        request = &mut local_to_remote => {
            request?;
            remote_to_local.await?;
            Ok(((), ()))
        }
    }
}

async fn send_local_request(
    local_read: &mut ReadHalf<'_>,
    iroh_send: &mut SendStream,
    event_sender: &Sender<Event>,
    session_id: &str,
) -> std::result::Result<(), anyhow::Error> {
    info!("TCP session {session_id}: forwarding local request started");
    let mut buffer = [0_u8; 8192];
    let mut bytes_forwarded = 0_u64;
    loop {
        let bytes_read = match local_read.read(&mut buffer).await {
            Ok(0) => break,
            Ok(bytes_read) => bytes_read,
            Err(err) => {
                log::error!("TCP session {session_id}: local TCP read failed: {err}");
                handle_inference_error(
                    event_sender,
                    "Failed reading local request",
                    &err.to_string(),
                )
                .await;
                return Err(err.into());
            }
        };

        if let Err(err) = iroh_send.write_all(&buffer[..bytes_read]).await {
            log::error!("TCP session {session_id}: QUIC request write failed: {err}");
            handle_inference_error(
                event_sender,
                "Failed writing local request to peer",
                &err.to_string(),
            )
            .await;
            return Err(err.into());
        }
        bytes_forwarded += bytes_read as u64;
    }
    info!("TCP session {session_id}: local request reached EOF after {bytes_forwarded} bytes");

    // TODO:  shoule we do this?, does this disconnect smthng?
    if let Err(err) = iroh_send.finish() {
        handle_inference_error(
            &event_sender,
            "Failed closing the conn after sending",
            &err.to_string(),
        )
        .await;
        return Err(err.into());
    }
    info!("TCP session {session_id}: QUIC request send stream finished");

    Result::<()>::Ok(())
}

async fn accept_remote_response(
    local_write: &mut WriteHalf<'_>,
    iroh_recv: &mut RecvStream,
    event_sender: &Sender<Event>,
    session_id: &str,
) -> std::result::Result<(), anyhow::Error> {
    info!("TCP session {session_id}: forwarding peer response started");
    match copy(iroh_recv, local_write).await {
        Ok(bytes) => {
            info!("TCP session {session_id}: peer response reached EOF after {bytes} bytes")
        }
        Err(err) => {
            log::error!("TCP session {session_id}: peer response copy failed: {err}");
            handle_inference_error(
                &event_sender,
                "Failed forwarding peer response to local",
                &err.to_string(),
            )
            .await;
            return Err(err.into());
        }
    }

    if let Err(err) = local_write.shutdown().await {
        handle_inference_error(
            &event_sender,
            "Failed closing local TCP writer",
            &err.to_string(),
        )
        .await;
        return Err(err.into());
    }
    info!("TCP session {session_id}: local response write half shut down");

    Result::<()>::Ok(())
}

async fn handle_inference_error(event_sender: &Sender<Event>, err_reason: &str, err_detail: &str) {
    let data = json!(
        {
            "reason": err_reason,
            "detail": err_detail
        }
    );
    log::error!("{}", data);
    let _ = event_sender.send(event("error", data)).await;
}

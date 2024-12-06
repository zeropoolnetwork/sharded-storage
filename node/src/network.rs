use std::{collections::HashMap, sync::Arc, time::Duration};

use color_eyre::{eyre::Error, Result};
use libp2p::{
    futures::StreamExt,
    identity,
    multiaddr::Protocol,
    request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};

use crate::state::{AppState, NodeId, Peer};

#[derive(Debug, Clone)]
pub struct Config {
    pub p2p_port: u16,
    pub boot_node: Option<Multiaddr>,
    pub node_id: NodeId,
    pub public_api_url: String,
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    request_response: request_response::cbor::Behaviour<Req, Res>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum NodeKind {
    Validator,
    Storage { id: NodeId },
}

#[derive(Debug, Serialize, Deserialize)]
enum Req {
    // TODO: Implement a custom handshake protocol instead of piggybacking on the request-response protocol.
    // This is an ad-hoc peer discovery solution. Instead of using a DHT, we use a replicated routing table.
    /// Ask a bootstrap node to initialize us.
    InitNode { kind: NodeKind, api_url: String },
    /// Notify peers about the new node. An ad-hoc peer discovery solution.
    NewNode { kind: NodeKind, peer: Peer },

    // TODO: Separate this from the basic network messages.
    /// A request to upload a cluster.
    UploadCluster { index: usize, data: Vec<u8> },
}

#[derive(Debug, Serialize, Deserialize)]
enum Res {
    /// The reply to an `InitNode` request. Contains needed network information.
    InitNodeSuccess {
        peers: HashMap<NodeId, Peer>,
    },
    InitNodeFailure {
        error: String,
    },
    NewNodeAcknowledged,
}

pub async fn start_network(config: Config, state: Arc<AppState>) -> Result<()> {
    let local_key = load_or_generate_keypair(&format!("data/node{}-keypair", config.node_id))?;

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_quic()
        .with_dns()?
        .with_behaviour(|_key| Behaviour {
            request_response: request_response::cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/zpss/1"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm.listen_on(format!("/ip4/0.0.0.0/udp/{}/quic-v1", config.p2p_port).parse()?)?;

    if let Some(multiaddr) = config.boot_node {
        tracing::info!("Bootstrapping from {}", multiaddr);
        let peer = extract_peer_id_from_addr(&multiaddr)?;
        swarm.add_peer_address(peer, multiaddr);
        swarm.behaviour_mut().request_response.send_request(
            &peer,
            Req::InitNode {
                kind: NodeKind::Storage { id: config.node_id },
                api_url: config.public_api_url.clone(),
            },
        );
    }

    let cloned_state = state.clone();
    let state = cloned_state;

    // FIXME: Implement cleanup
    let mut address_cache = HashMap::new();

    loop {
        let event = swarm.select_next_some().await;

        let res: Result<()> = (|| async {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    tracing::info!(
                        "{}",
                        address.clone().with(Protocol::P2p(*swarm.local_peer_id()))
                    );
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id, endpoint, ..
                } => {
                    // TODO: Should work with the QUIC transport. Do we care about TCP?
                    //       Maybe just include the remote address in the handshake/init message.
                    let remote_addr = endpoint.get_remote_address();
                    address_cache.insert(peer_id, remote_addr.clone());
                    tracing::debug!("Connected to peer {} at {}", peer_id, remote_addr);
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    tracing::debug!("Disconnected from {}: {:?}", peer_id, cause);
                }
                SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                    request_response::Event::ResponseSent { .. },
                )) => {
                    tracing::trace!("Request sent");
                }
                SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                    ev @ request_response::Event::InboundFailure { .. },
                )) => {
                    tracing::error!("Inbound failure: {:?}", ev);
                }
                SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                    ev @ request_response::Event::OutboundFailure { .. },
                )) => {
                    tracing::error!("Outbound failure: {:?}", ev);
                }
                SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                    request_response::Event::Message { message, peer },
                )) => match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        tracing::debug!("Request from {}: {:?}", peer, request);
                        match request {
                            Req::InitNode { kind, api_url } => {
                                match kind {
                                    NodeKind::Storage { id } => {
                                        if state.peers.read().await.contains_key(&id) {
                                            let _ = swarm
                                                .behaviour_mut()
                                                .request_response
                                                .send_response(
                                                    channel,
                                                    Res::InitNodeSuccess {
                                                        peers: state.peers.read().await.clone(),
                                                    },
                                                );
                                            return Ok(());
                                        }
                                    }
                                    NodeKind::Validator => {
                                        if state
                                            .validators
                                            .read()
                                            .await
                                            .iter()
                                            .any(|p| p.peer_id == peer)
                                        {
                                            let _ = swarm
                                                .behaviour_mut()
                                                .request_response
                                                .send_response(
                                                    channel,
                                                    Res::InitNodeSuccess {
                                                        peers: state.peers.read().await.clone(),
                                                    },
                                                );
                                            return Ok(());
                                        }
                                    }
                                }

                                let _ = swarm.behaviour_mut().request_response.send_response(
                                    channel,
                                    Res::InitNodeSuccess {
                                        peers: state.peers.read().await.clone(),
                                    },
                                );

                                match address_cache.get(&peer) {
                                    Some(peer_addr) => {
                                        let peer = Peer {
                                            peer_id: peer,
                                            addr: peer_addr.clone(),
                                            api_url,
                                        };
                                        match kind {
                                            NodeKind::Storage { id } => {
                                                state.peers.write().await.insert(id, peer);
                                            }
                                            NodeKind::Validator => {
                                                state.validators.write().await.push(peer);
                                            }
                                        }
                                    }
                                    None => {
                                        let _ = swarm.disconnect_peer_id(peer);
                                        tracing::error!("InitNode req: peer address not cached");
                                    }
                                }

                                // TODO: Given that we have a full-mesh topology, is it ok to use
                                //       request_response for broadcasting here or use gossipsub instead?
                                for peer in state
                                    .peers
                                    .read()
                                    .await
                                    .values()
                                    .filter(|p| p.peer_id != peer)
                                    .chain(state.validators.read().await.iter())
                                {
                                    swarm.behaviour_mut().request_response.send_request(
                                        &peer.peer_id,
                                        Req::NewNode {
                                            kind: kind.clone(),
                                            peer: peer.clone(),
                                        },
                                    );
                                }
                            }
                            Req::UploadCluster { index, data, .. } => {
                                state.storage.write(index, &data).await?;
                            }
                            Req::NewNode { kind, peer } => {
                                swarm.add_peer_address(peer.peer_id.clone(), peer.addr.clone());
                                match kind {
                                    NodeKind::Storage { id } => {
                                        state.peers.write().await.insert(id, peer);
                                    }
                                    NodeKind::Validator => {
                                        state.validators.write().await.push(peer);
                                    }
                                }
                                let _ = swarm
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, Res::NewNodeAcknowledged);
                            }
                        }
                    }
                    request_response::Message::Response { response, .. } => {
                        tracing::debug!("Response from {}: {:?}", peer, response);
                        match response {
                            Res::InitNodeSuccess { peers } => {
                                state.peers.write().await.extend(peers);
                            }
                            Res::InitNodeFailure { error } => {
                                // TODO: Retry
                                panic!("InitNode failed: {}", error);
                            }
                            Res::NewNodeAcknowledged => {
                                tracing::debug!("New node acknowledged by {}", peer);
                            }
                        }
                    }
                },

                _ => {}
            }

            Ok(())
        })()
        .await;

        if let Err(err) = res {
            tracing::error!("Event processing failed: {}", err);
        }
    }
}

fn extract_peer_id_from_addr(addr: &Multiaddr) -> Result<PeerId> {
    addr.iter()
        .find_map(|addr| match addr {
            Protocol::P2p(key) => Some(key),
            _ => None,
        })
        .ok_or_else(|| Error::msg("No peer ID in bootstrap address"))
}

fn load_or_generate_keypair(path: &str) -> Result<identity::Keypair> {
    let keypair = match std::fs::read(path) {
        Ok(data) => identity::Keypair::from_protobuf_encoding(&data)?,
        Err(_) => {
            let keypair = identity::Keypair::generate_ed25519();
            std::fs::write(path, keypair.to_protobuf_encoding()?)?;
            keypair
        }
    };

    Ok(keypair)
}

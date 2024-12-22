use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use color_eyre::{eyre::Error, Result};
use libp2p::{
    futures::StreamExt,
    identity,
    multiaddr::Protocol,
    request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol, Swarm,
};
use primitives::Val;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use common::contract::ClusterId;
use crate::state::{AppState, Command, NodeId, NodeKind, NodeState, Peer};

#[derive(Debug, Clone)]
pub struct Config {
    pub p2p_port: u16,
    pub boot_node: Option<Multiaddr>,
    pub node_kind: NodeKind,
    pub public_api_url: String,
    pub external_ip: String,
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    request_response: request_response::cbor::Behaviour<Req, Res>,
}

#[derive(Debug, Serialize, Deserialize)]
enum Req {
    // TODO: Implement a custom handshake protocol instead of piggybacking on the request-response protocol.
    //       Or maybe use kademlia for peer discovery with a full local cache for each node.
    // This is an ad-hoc peer discovery solution. Instead of using a DHT, we use a replicated routing table.
    /// Ask a bootstrap node to initialize us.
    InitNode {
        kind: NodeKind,
        api_url: String,
        external_addr: Multiaddr,
    },
    /// Notify peers about the new node. An ad-hoc peer discovery solution.
    NewNode { kind: NodeKind, peer: Peer },

    // TODO: Separate this from the basic network messages.
    /// A request to upload a cluster.
    UploadCluster { index: u64, id: ClusterId, data: Vec<Val> },

    DownloadCluster { index: u64 },
}

// TODO: Same naming for variants in Req and Res.
#[derive(Debug, Serialize, Deserialize)]
enum Res {
    /// The reply to an `InitNode` request. Contains needed network information.
    InitNodeSuccess {
        boot_node_kind: NodeKind,
        boot_node_public_api_url: String,
        validators: HashSet<Peer>,
        peers: HashMap<NodeId, Peer>,
    },
    InitNodeFailure {
        error: String,
    },
    UploadClusterSuccess,
    UploadClusterFailure,
    NewNodeAcknowledged,
    DownloadClusterSuccess(Vec<u8>),
    DownloadClusterFailure,
}

pub async fn start_network(
    config: Config,
    state: Arc<AppState>,
    mut command_receiver: mpsc::Receiver<Command>,
) -> Result<()> {
    let local_key = match config.node_kind {
        NodeKind::Validator => load_or_generate_keypair("data/validator-keypair"),
        NodeKind::Storage { id } => load_or_generate_keypair(&format!("data/node{}-keypair", id)),
    }?;

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

    // TODO: Proper NAT traversal
    let full_external_addr = format!(
        "/ip4/{}/udp/{}/quic-v1",
        config.external_ip, config.p2p_port
    )
    .parse::<Multiaddr>()?
    .with(Protocol::P2p(*swarm.local_peer_id()));

    // dump the address to file
    match config.node_kind {
        NodeKind::Validator => std::fs::write("data/validator_addr", full_external_addr.to_string())?,
        NodeKind::Storage { id } => std::fs::write(format!("data/node{}_addr", id), full_external_addr.to_string())?,
    }

    swarm.listen_on(format!("/ip4/0.0.0.0/udp/{}/quic-v1", config.p2p_port).parse()?)?;

    if let Some(multiaddr) = &config.boot_node {
        tracing::info!("Bootstrapping from {}", multiaddr);
        let peer = extract_peer_id_from_addr(&multiaddr)?;
        swarm.add_peer_address(peer, multiaddr.clone());
        swarm.behaviour_mut().request_response.send_request(
            &peer,
            Req::InitNode {
                kind: config.node_kind.clone(),
                api_url: config.public_api_url.clone(),
                external_addr: full_external_addr,
            },
        );
    }

    let cloned_state = state.clone();
    let state = cloned_state;

    loop {
        // TODO: Check for heavy blockers inside of the loop
        let res: Result<()> = tokio::select! {
            event = swarm.select_next_some() => process_event(event, &mut swarm, state.clone(), &config).await,
            command = command_receiver.recv() => process_command(command, &mut swarm, state.clone()).await,
        };

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
            std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap())?;

            let keypair = identity::Keypair::generate_ed25519();
            std::fs::write(path, keypair.to_protobuf_encoding()?)?;
            keypair
        }
    };

    Ok(keypair)
}

async fn process_event(
    event: SwarmEvent<BehaviourEvent>,
    swarm: &mut Swarm<Behaviour>,
    state: Arc<AppState>,
    config: &Config,
) -> Result<()> {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            tracing::info!(
                "{}",
                address.clone().with(Protocol::P2p(*swarm.local_peer_id()))
            );
        }
        SwarmEvent::IncomingConnection {
            local_addr,
            send_back_addr,
            ..
        } => {
            tracing::debug!(
                "Incoming connection from {} to {}",
                send_back_addr,
                local_addr
            );
        }
        SwarmEvent::IncomingConnectionError { error, .. } => {
            tracing::error!("Incoming connection error: {:?}", error);
        }
        SwarmEvent::OutgoingConnectionError { error, .. } => {
            tracing::error!("Outgoing connection error: {:?}", error);
        }
        SwarmEvent::ConnectionEstablished {
            peer_id, endpoint, ..
        } => {
            let remote_addr = endpoint.get_remote_address();
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
                tracing::debug!("Request from {}", peer);
                match request {
                    Req::InitNode {
                        kind,
                        api_url,
                        external_addr,
                    } => {
                        let mut peers = state.peers.write().await;
                        let mut validators = state.validators.write().await;

                        let peer_data = Peer {
                            peer_id: peer,
                            addr: external_addr.clone(),
                            api_url: api_url.clone(),
                        };

                        let (v, p) = match kind {
                            NodeKind::Storage { id } => {
                                peers.insert(id, peer_data.clone());
                                (
                                    validators.clone(),
                                    peers
                                        .iter()
                                        .map(|(k, v)| (*k, v.clone()))
                                        .filter(|(k, _)| *k != id)
                                        .collect::<HashMap<_, _>>(),
                                )
                            }
                            NodeKind::Validator => {
                                validators.insert(peer_data.clone());
                                (
                                    validators
                                        .iter()
                                        .cloned()
                                        .filter(|p| p.peer_id != peer)
                                        .collect(),
                                    peers.clone(),
                                )
                            }
                        };

                        let _ = swarm.behaviour_mut().request_response.send_response(
                            channel,
                            Res::InitNodeSuccess {
                                boot_node_kind: config.node_kind.clone(),
                                boot_node_public_api_url: config.public_api_url.clone(),
                                validators: v,
                                peers: p,
                            },
                        );

                        swarm.add_peer_address(peer, external_addr);

                        // Notify all other nodes about the new node.
                        // Given that we have a full-mesh topology it should be ok to use request-response
                        // for broadcasting.
                        for peer in peers
                            .values()
                            .chain(validators.iter())
                            .filter(|p| p.peer_id != peer)
                        {
                            swarm.behaviour_mut().request_response.send_request(
                                &peer.peer_id,
                                Req::NewNode {
                                    kind: kind.clone(),
                                    peer: peer_data.clone(),
                                },
                            );
                        }
                    }
                    Req::UploadCluster { index, mut data, id } => match &state.node_state {
                        NodeState::Validator => {
                            tracing::warn!("Ignoring UploadCluster request in validator mode");
                            let _ = swarm
                                .behaviour_mut()
                                .request_response
                                .send_response(channel, Res::UploadClusterFailure);
                        }
                        NodeState::Storage { storage } => {
                            tracing::info!("Writing cluster {}", index);

                            // Safety: Vec<Val> can be safely converted to Vec<u8> with length and capacity adjusted.
                            let data = unsafe {
                                data[..].align_to::<u8>().1.to_vec()
                            };

                            storage.write(index as usize, &data).await?;

                            let _ = swarm
                                .behaviour_mut()
                                .request_response
                                .send_response(channel, Res::UploadClusterSuccess);

                            state.cluster_id_cache.write().await.insert(id, index as usize);
                        }
                    },
                    Req::NewNode { kind, peer } => {
                        swarm.add_peer_address(peer.peer_id.clone(), peer.addr.clone());
                        match kind {
                            NodeKind::Storage { id } => {
                                tracing::info!("New storage node connected ({})", id);
                                state.peers.write().await.insert(id, peer);
                            }
                            NodeKind::Validator => {
                                tracing::info!("New validator connected");
                                state.validators.write().await.insert(peer);
                            }
                        }
                        let _ = swarm
                            .behaviour_mut()
                            .request_response
                            .send_response(channel, Res::NewNodeAcknowledged);
                    }
                    Req::DownloadCluster { index } => {
                        tracing::debug!("Downloading cluster {}", index);
                        match &state.node_state {
                            NodeState::Validator => {
                                tracing::warn!("Ignoring DownloadCluster request in validator mode");
                                let _ = swarm
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, Res::DownloadClusterFailure);
                            }
                            NodeState::Storage { storage } => {
                                let data = storage.read(1, index as usize).await?;
                                let _ = swarm
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, Res::DownloadClusterSuccess(data));
                            }
                        }
                    }
                }
            }
            request_response::Message::Response { response, .. } => {
                tracing::info!("Response from {}: {:?}", peer, response);
                match response {
                    Res::InitNodeSuccess {
                        boot_node_kind,
                        boot_node_public_api_url,
                        validators,
                        peers,
                    } => {
                        let mut local_peers = state.peers.write().await;
                        let mut local_validators = state.validators.write().await;

                        local_peers.extend(peers.into_iter());
                        local_validators.extend(validators.into_iter());

                        match boot_node_kind {
                            NodeKind::Storage { id } => {
                                local_peers.insert(
                                    id,
                                    Peer {
                                        peer_id: peer,
                                        addr: config.boot_node.clone().unwrap(),
                                        api_url: boot_node_public_api_url,
                                    },
                                );
                            }
                            NodeKind::Validator => {
                                local_validators.insert(Peer {
                                    peer_id: peer,
                                    addr: config.boot_node.clone().unwrap(),
                                    api_url: boot_node_public_api_url,
                                });
                            }
                        }

                        tracing::info!("Node initialized successfully");
                    }
                    Res::InitNodeFailure { error } => {
                        panic!("InitNode failed: {}", error);
                    }
                    Res::NewNodeAcknowledged => {
                        tracing::debug!("New node acknowledged by {}", peer);
                    }
                    Res::UploadClusterSuccess => {
                        tracing::debug!("Cluster uploaded successfully");
                    }
                    Res::UploadClusterFailure => {
                        tracing::error!("Cluster upload failed");
                    }
                    _ => {}
                }
            }
        },

        _ => {}
    }

    Ok(())
}

async fn process_command(
    command: Option<Command>,
    swarm: &mut Swarm<Behaviour>,
    state: Arc<AppState>,
) -> Result<()> {
    match command {
        Some(Command::UploadCluster { index, id, shards }) => {
            let peers = state.peers.read().await;

            for (shard_index, shard) in shards.into_iter().enumerate() {
                let peer = peers
                    .get(&(shard_index as u32))
                    .ok_or_else(|| Error::msg("Peer not found"))?;
                swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer.peer_id, Req::UploadCluster { index, id: id.clone(), data: shard });
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

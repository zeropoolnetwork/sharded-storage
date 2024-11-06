use std::{collections::HashMap, sync::Arc, time::Duration};

use color_eyre::{eyre::Error, Result};
use libp2p::{
    futures::StreamExt,
    identity, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
// use crate::state::{AppState, NodeId, Peer};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub peer_id: PeerId,
    pub addr: Multiaddr,
    pub api_url: String,
}

pub type NodeId = u32;

pub struct AppState {
    // We don't need to use kademlia since our routing table is pretty small, and we need to access
    // nodes by their ID with as little delay as possible. So storing the full routing table on each
    // node with some ad-hoc replication is acceptable for now.
    /// Routing table that also contains some node metadata.
    pub peers: RwLock<HashMap<NodeId, Peer>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
enum Req {
    // TODO: Implement a custom handshake protocol instead of piggybacking on the request-response protocol.
    /// Ask a bootstrap node to initialize us.
    InitNode { id: NodeId, api_url: String },

    /// A request to upload a sector.
    Upload {
        index: usize,
        data: Vec<u8>,
        // signature: Vec<u8>, // FIXME: proper signature type
    },

    // TODO: Try using validator nodes as a rendezvous points.
    /// Notify peers about the new node. An ad-hoc peer discovery solution.
    NewNode { id: NodeId, peer: Peer },
}

#[derive(Debug, Serialize, Deserialize)]
enum Res {
    /// The reply to an `InitNode` request. Contains needed network information.
    InitNode { peers: HashMap<NodeId, Peer> },
}

pub async fn start_network(config: Config, state: Arc<AppState>) -> Result<()> {
    let local_key = identity::Keypair::generate_ed25519();

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_quic()
        .with_dns()?
        .with_behaviour(|_key| Behaviour {
            request_response: request_response::cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/test/1"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm.listen_on(format!("/ip4/0.0.0.0/udp/{}/quic-v1", config.p2p_port).parse()?)?;

    if let Some(multiaddr) = config.boot_node {
        let peer = extract_peer_id_from_addr(&multiaddr)?;
        swarm.add_peer_address(peer, multiaddr);
        swarm.behaviour_mut().request_response.send_request(
            &peer,
            Req::InitNode {
                id: config.node_id,
                api_url: config.public_api_url.clone(),
            },
        );
    }

    let cloned_state = state.clone();
    let state = cloned_state;
    let mut peer_cache = HashMap::new();

    loop {
        let event = swarm.select_next_some().await;

        let res: Result<()> = (|| async {
            match event {
                SwarmEvent::ConnectionEstablished {
                    peer_id, endpoint, ..
                } => {
                    let remote_addr = endpoint.get_remote_address(); // FIXME: Do we care about cases where this wouldn't work?
                    peer_cache.insert(peer_id, remote_addr.clone());
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
                    request_response::Event::Message { message, peer },
                )) => match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => match request {
                        Req::InitNode { id, api_url } => {
                            let _ = swarm.behaviour_mut().request_response.send_response(
                                channel,
                                Res::InitNode {
                                    peers: state.peers.read().await.clone(),
                                },
                            );

                            match peer_cache.get(&peer) {
                                Some(peer_addr) => {
                                    let peer = Peer {
                                        peer_id: peer,
                                        addr: peer_addr.clone(),
                                        api_url,
                                    };
                                    state.peers.write().await.insert(id, peer);
                                }
                                None => {
                                    let _ = swarm.disconnect_peer_id(peer);
                                    tracing::error!("InitNode req: peer address not cached");
                                }
                            }

                            // TODO: Given that we have a full-mesh topology, is it ok to use
                            //       request_response for broadcasting here or just implement gossipsub?
                            for peer in state.peers.read().await.values() {
                                swarm.behaviour_mut().request_response.send_request(
                                    &peer.peer_id,
                                    Req::NewNode {
                                        id,
                                        peer: peer.clone(),
                                    },
                                );
                            }
                        }
                        Req::Upload { .. } => {
                            tracing::debug!("Ignoring upload request");
                        }
                        Req::NewNode { id, peer } => {
                            swarm.add_peer_address(peer.peer_id.clone(), peer.addr.clone());
                            state.peers.write().await.insert(id, peer);
                        }
                    },
                    request_response::Message::Response { response, .. } => match response {
                        Res::InitNode { peers } => {
                            state.peers.write().await.extend(peers);
                        }
                    },
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
            libp2p::core::multiaddr::Protocol::P2p(key) => Some(key),
            _ => None,
        })
        .ok_or_else(|| Error::msg("No peer ID in bootstrap address"))
}

use env_logger::{Builder, Env};
use futures::{future, prelude::*};
use libp2p::gossipsub::protocol::MessageId;
use libp2p::gossipsub::{GossipsubEvent, GossipsubMessage, Topic};
use libp2p::{gossipsub, identity, PeerId};
use std::collections::hash_map::DefaultHasher;
use std::time::Duration;
use std::hash::{Hash, Hasher};
use std::{error::Error, task::{Context, Poll}};

fn gossipsub_chat() -> Result<(), Box<dyn Error>> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("local peer id: {:?}", local_peer_id);

    
}
use futures::stream::{StreamExt, TryStreamExt};
use libp2p::{
    core::upgrade, identity::Keypair, NetworkBehaviour, PeerId, 
    secio::SecioConfig, Swarm, Transport, tcp::TcpConfig, yamux, 
    gossipsub::{self, protocol::MessageId, GossipsubEvent, GossipsubMessage, Gossipsub},
    swarm::{self, NetworkBehaviourEventProcess}
};
use pin_project::pin_project;
use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    future::Future,
    hash::{Hash, Hasher},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{self, AsyncBufReadExt};

#[derive(NetworkBehaviour)]
pub struct GossipsubBehaviour {
    protocol: Gossipsub, 
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for GossipsubBehaviour {
    fn inject_event(&mut self, message: GossipsubEvent) {
        if let GossipsubEvent::Message(peer_id, message_id, message) = message {
            println!(
                "Got message: {}, with id: {} from peer: {:?}",
                String::from_utf8_lossy(&message.data),
                message_id,
                peer_id
            )
        }
    }
}

#[pin_project]
struct SwarmFuture<T> where T: swarm::NetworkBehaviour {
    #[pin]
    swarm: Swarm<T>,
    #[pin]
    stdin: io::Lines<io::BufReader<io::Stdin>>,
    topic: gossipsub::Topic,
    listening: bool
}

impl SwarmFuture<GossipsubBehaviour> {
    fn new(swarm: Swarm<GossipsubBehaviour>, stdin: io::Lines<io::BufReader<io::Stdin>>, topic: gossipsub::Topic) -> Self {
        Self {
            swarm,
            stdin,
            topic,
            listening: false
        }
    }
}

impl Future for SwarmFuture<GossipsubBehaviour> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let listening = this.listening;
        let mut swarm: Pin<&mut _> = this.swarm;
        let mut stdin = this.stdin;
        let topic = this.topic;

        loop {
            match stdin.try_poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(line))) => swarm.protocol.publish(&topic, line.as_bytes()),
                Poll::Ready(Some(Err(_))) | Poll::Ready(None) => panic!("Stdin closed"),
                Poll::Pending => break,
            }
        }

        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => println!("current event: {:?}", event),
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => {
                    if !*listening {
                        for addr in Swarm::listeners(&swarm) {
                            println!("Listening on {:?}", addr);
                            *listening = true;
                        }
                    }
                    break;
                }
            }
        }
        Poll::Pending
    }
}

pub async fn gossipsub_chat() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let local_key = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("local peer id: {:?}", local_peer_id);

    let transport = {
        let tcp_config = TcpConfig::new();
        let secio_config = SecioConfig::new(Keypair::generate_ed25519());
        let yamux = yamux::Config::default();

        tcp_config.upgrade(upgrade::Version::V1).authenticate(secio_config).multiplex(yamux)
    };

    let topic = gossipsub::Topic::new("test-net".into());

    let mut swarm = {
        let mut behaviour = {
            let message_id_fn = |message: &GossipsubMessage| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId(s.finish().to_string())
            };
    
            let gossipsub_config = gossipsub::GossipsubConfigBuilder::new()
                .heartbeat_interval(Duration::from_secs(10))
                .message_id_fn(message_id_fn)
                .build();
    
            let gossipsub = gossipsub::Gossipsub::new(local_peer_id.clone(), gossipsub_config);
            GossipsubBehaviour {
                protocol: gossipsub
            }
        };
        behaviour.protocol.subscribe(topic.clone());
        libp2p::Swarm::new(transport, behaviour, local_peer_id)
    };

    libp2p::Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9944".parse().unwrap()).unwrap();

    // if let Some(to_dial) = std::env::args().nth(1) {
    //     let dialing = to_dial.clone();
    //     match to_dial.parse() {
    //         Ok(to_dial) => match libp2p::Swarm::dial_addr(&mut swarm, to_dial) {
    //             Ok(_) => println!("Dialed: {:?}", dialing),
    //             Err(e) => println!("Dialed: {:?} failed: {:?}", dialing, e),
    //         }
    //         Err(e) => println!("Failed to parse address to dial: {:?}", e),
    //     }
    // }
    if let Some(addr) = std::env::args().nth(1) {
        let remote = addr.parse()?;
        Swarm::dial_addr(&mut swarm, remote)?;
        println!("Dialed {}", addr);
    }

    let stdin = io::BufReader::new(io::stdin()).lines();

    Ok(SwarmFuture::new(swarm, stdin, topic).await)
}

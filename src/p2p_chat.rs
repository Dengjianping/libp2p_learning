use futures::prelude::*;
use futures::stream::StreamExt;
use libp2p::{
    Multiaddr,
    PeerId,
    Swarm,
    swarm,
    NetworkBehaviour,
    identity,
    floodsub::{self, Floodsub, FloodsubEvent},
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess
};
use std::{error::Error, task::{Context, Poll}};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{self, AsyncBufReadExt};
use pin_project::pin_project;

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,

    #[behaviour(ignore)]
    #[allow(dead_code)]
    ignore_member: bool,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            println!("Received: {:?} from {:?}", String::from_utf8_lossy(&message.data), message.source);
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    self.floodsub.remove_node_from_partial_view(&peer);
                }
            }
        }
    }
}

#[pin_project]
struct SwarmFuture<T> where T: swarm::NetworkBehaviour {
    #[pin]
    swarm: Swarm<T>,
    #[pin]
    stdin: io::Lines<io::BufReader<io::Stdin>>,
    topic: floodsub::Topic,
    listening: bool
}

impl SwarmFuture<MyBehaviour> {
    fn new(swarm: Swarm<MyBehaviour>, stdin: io::Lines<io::BufReader<io::Stdin>>, topic: &str) -> Self {
        let topic = floodsub::Topic::new(topic);
        Self {
            swarm,
            stdin,
            topic,
            listening: false
        }
    }
}

impl Future for SwarmFuture<MyBehaviour> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let mut listening = this.listening;
        let mut swarm: Pin<&mut _> = this.swarm;
        let mut stdins = this.stdin;
        let topic = this.topic;

        loop {
            match stdins.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(buf))) => {
                    println!("inputed: {}", buf);
                    println!("topic: {:?}", topic);
                    swarm.floodsub.publish(topic.clone(), buf.as_bytes());
                    println!("published: {}", 12);
                }
                Poll::Ready(None) | Poll::Ready(Some(Err(_))) => panic!("closed command line input."),
                Poll::Pending => break,
            }
        }

        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => println!("{:?}", 1234u32),
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

pub async fn p2p_chat() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let transport = libp2p::build_development_transport(local_key)?;

    let floodsub_topic = floodsub::Topic::new("chat");

    let mut swarm = {
        let mdns = Mdns::new()?;
        let mut behaviour = MyBehaviour {
            floodsub: Floodsub::new(local_peer_id.clone()),
            mdns, ignore_member: false,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());
        Swarm::new(transport, behaviour, local_peer_id)
    };

    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        println!("args: {:?}", std::env::args());
        Swarm::dial_addr(&mut swarm, addr)?;
        println!("Dialed {:?}", to_dial);
    }

    let stdin = io::BufReader::new(io::stdin()).lines();
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9944".parse()?)?;

    Ok(SwarmFuture::new(swarm, stdin, "chat").await)
}

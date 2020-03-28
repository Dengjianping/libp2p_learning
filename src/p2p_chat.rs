use futures::{future, prelude::*};
use futures::stream::{TryStream, TryStreamExt};
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
use tokio::io::{self, AsyncBufRead, AsyncBufReadExt};
use tokio::io::AsyncRead;
use pin_project::pin_project;

#[derive(NetworkBehaviour)]
struct MyBehaviour {
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
    // stdin: io::Lines<String>,
    stdin: io::Stdin,
    topic: floodsub::Topic,
    listening: bool
}

impl<T> SwarmFuture<T> where T: swarm::NetworkBehaviour {
    fn new(swarm: Swarm<T>, stdin: io::Stdin, topic: &str) -> Self {
        let topic = floodsub::Topic::new(topic);
        Self {
            swarm,
            stdin,
            topic,
            listening: false
        }
    }
}

impl<T> Future for SwarmFuture<T> where T: swarm::NetworkBehaviour {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let mut listening = this.listening;
        // let mut swarm: Pin<&mut _> = this.swarm;
        let mut swarm = this.swarm;
        let mut stdin: Pin<&mut _> = this.stdin;
        let topic = this.topic;

        loop {
            let mut buf = vec![];
            match stdin.poll_read(cx, &mut buf) {
                Poll::Ready(Ok(_)) => swarm.get_mut().floodsub.publish(topic.clone(), &buf),
                Poll::Ready(Err(_)) => panic!("closed command line input."),
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

pub async fn p2p_chat1() -> Result<(), Box<dyn Error>> {
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
        // let addr = "/ip4/192.168.31.204/tcp/9944".parse()?;
        println!("args: {:?}", std::env::args());
        Swarm::dial_addr(&mut swarm, addr);
        println!("Dialed {:?}", to_dial);
    }

    // let mut stdin = io::BufReader::new(io::stdin());
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9944".parse()?)?;

    Ok(SwarmFuture::new(swarm, io::stdin(), "chat").await)
}

pub async fn p2p_chat() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let transport = libp2p::build_development_transport(local_key)?;

    let floodsub_topic = floodsub::Topic::new("char");

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
        // let addr = "/ip4/192.168.31.204/tcp/9944".parse()?;
        println!("args: {:?}", std::env::args());
        Swarm::dial_addr(&mut swarm, addr);
        println!("Dialed {:?}", to_dial);
    }

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9944".parse()?)?;

    let mut listening = false;
    let fut = future::poll_fn(move |cx: &mut Context| {
        loop {
            match stdin.try_poll_next_unpin(cx)? {
                Poll::Ready(Some(line)) => swarm.floodsub.publish(floodsub_topic.clone(), line.as_bytes()),
                Poll::Ready(None) => panic!("Stdin closed"),
                Poll::Pending => break
            }
        }

        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => println!("{:?}", event),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => {
                    if !listening {
                        for addr in Swarm::listeners(&swarm) {
                            println!("Listening on {:?}", addr);
                            listening = true;
                        }
                    }
                    break;
                }
            }
        }
        Poll::Pending
    });
    fut.await
}

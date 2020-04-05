use futures::prelude::*;
use libp2p::{identity, PeerId, ping::{Ping, PingEvent, PingConfig}, Swarm};
use libp2p::swarm::{NetworkBehaviourEventProcess, NetworkBehaviour};
use std::{
    error::Error,
    task::{Context, Poll},
    future::Future,
    pin::Pin
};
use pin_project::pin_project;

// #[derive(libp2p::NetworkBehaviour)]
// pub struct PingBehavior {
//     ping: Ping
// }

// impl NetworkBehaviourEventProcess<PingEvent> for PingBehavior {
//     fn inject_event(&mut self, message: PingEvent) {
//         let PingEvent {peer, result} = message;
//         match result {
//             Ok(_) => println!("got message from peer"),
//             Err(_) => println!("lost connection from peer"),
//         }
//     }
// }

#[pin_project]
struct SwarmFuture<T> where T: NetworkBehaviour {
    #[pin]
    swarm: Swarm<T>,
    listening: bool
}

impl SwarmFuture<Ping> {
    fn new(swarm: Swarm<Ping>) -> Self {
        Self {
            swarm,
            listening: false
        }
    }
}

impl Future for SwarmFuture<Ping> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let listening: &mut bool = this.listening;
        let mut pined_swarm: Pin<&mut _> = this.swarm;
        loop {
            match pined_swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => {
                    println!("current event: {:?}", event)
                }
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => {
                    if !*listening {
                        for addr in Swarm::listeners(&pined_swarm) {
                            println!("Listening on {}", addr);
                            *listening = true;
                        }
                    }
                    return Poll::Pending
                }
            }
        }
    }
}

pub async fn p2p_ping() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("local peer id: {:?}", peer_id);

    let transport = libp2p::build_development_transport(id_keys)?;

    let behavior = {
        let ping = Ping::new(PingConfig::new().with_keep_alive(true));
        // PingBehavior { ping }
        ping
    };

    let mut swarm = Swarm::new(transport, behavior, peer_id);

    if let Some(addr) = std::env::args().nth(1) {
        let remote = addr.parse()?;
        Swarm::dial_addr(&mut swarm, remote)?;
        println!("Dialed {}", addr);
    }

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9944".parse()?)?;

    Ok(
        SwarmFuture::new(swarm).await
    )

}

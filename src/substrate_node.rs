use futures::stream::StreamExt;
use libp2p::{
    Swarm,
    swarm,
    PeerId,
    identity,
    build_development_transport,
    kad::{
        Kademlia, KademliaConfig, KademliaEvent, GetClosestPeersError,
        record::store::MemoryStore,
    }
};
use std::{
    env, error::Error, time::Duration, str::FromStr, task::{Poll, Context},
};

pub async fn connect_node() -> Result<(), Box<dyn Error>> {

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("peer id: {:?}", local_peer_id);

    let transport = build_development_transport(local_key).unwrap();

    let mut swarm = {
        let protocol = b"sup";
        let mut cfg = KademliaConfig::default();//.set_protocol_name(protocol.into());
        cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store = MemoryStore::new(local_peer_id.clone());
        let mut behaviour = Kademlia::with_config(local_peer_id.clone(), store, cfg);

        behaviour.add_address(
            // &PeerId::from_str("mXdLF5wTMLJ6YCiNWvgXDdf52c3sqnb2acyHy4nYPjmb8").unwrap(),
            &"QmXdLF5wTMLJ6YCiNWvgXDdf52c3sqnb2acyHy4nYPjmb8".parse().unwrap(),
            "/ip4/127.0.0.1/tcp/30333".parse().unwrap()
        );
        
        Swarm::new(transport, behaviour, local_peer_id)
    };
    Swarm::dial_addr(&mut swarm, "/ip4/127.0.0.1/tcp/30333".parse().unwrap())?;

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/1234".parse()?)?;

    let fut = futures::future::poll_fn(move |cx: &mut Context| {
        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => {
                    // if let KademliaEvent::GetClosestPeersResult(result) = event {
                    //     match result {
                    //         Ok(ok) => {
                    //             if !ok.peers.is_empty() {
                    //                 println!("Query finished with closest peers: {:#?}", ok.peers)
                    //             } else {
                    //                 println!("Query finished with no closest peers.")
                    //             }
                    //         }
                    //         Err(GetClosestPeersError::Timeout { peers, .. }) => {
                    //             if !peers.is_empty() {
                    //                 println!("Query timed out with closest peers: {:#?}", peers)
                    //             } else {
                    //                 println!("Query time out with no closest peers.")
                    //             }
                    //         }
                    //     }
                    // }
                    match event {
                        KademliaEvent::Discovered{ peer_id, ..} => {
                            println!("peer id: {:?}", peer_id);
                        }
                        _ => println!("unknown event."),
                    }
                }
                Poll::Ready(None) => {
                    println!("no event triggered.");
                    return Poll::Ready(());
                }
                Poll::Pending => {
                    println!("received pending request.");
                    return Poll::Pending;
                }
            }
        }
    });

    let _ = fut.await;

    Ok(())
}
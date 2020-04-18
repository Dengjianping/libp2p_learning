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
    env, error::Error, time::Duration, str::FromStr,
};

pub async fn ipfs_kad() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let transport = build_development_transport(local_key).unwrap();

    let mut swarm = {
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store = MemoryStore::new(local_peer_id.clone());
        let mut behaviour = Kademlia::with_config(local_peer_id.clone(), store, cfg);

        behaviour.add_address(
            &PeerId::from_str("QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ").unwrap(),
            "/ip4/104.131.131.82/tcp/4001".parse()?
        );

        Swarm::new(transport, behaviour, local_peer_id)
    };

    let to_search: PeerId = if let Some(peer_id) = env::args().nth(1) {
        peer_id.parse()?
    } else {
        identity::Keypair::generate_ed25519().public().into()
    };

    println!("Search for the closest peers to {:?}", to_search);
    swarm.get_closest_peers(to_search);

    let fut = futures::executor::block_on(async move {
        loop {
            let event = swarm.next().await;

            if let KademliaEvent::GetClosestPeersResult(result) = event {
                match result {
                    Ok(ok) => {
                        if !ok.peers.is_empty() {
                            println!("Query finished with closest peers: {:#?}", ok.peers)
                        } else {
                            println!("Query finished with no closest peers.")
                        }
                    }
                    Err(GetClosestPeersError::Timeout { peers, .. }) => {
                        if !peers.is_empty() {
                            println!("Query timed out with closest peers: {:#?}", peers)
                        } else {
                            println!("Query time out with no closest peers.")
                        }
                    }
                }
            }

            break;
        }
    });

    // let _ = fut.await;
    Ok(())
}
mod p2p_ping;
mod p2p_chat;
mod p2p_gossipsub;
mod ipfs_kad;
mod substrate_node;

#[tokio::main]
async fn main() {
    // let _ = p2p_ping::p2p_ping().await;
    // let _ = p2p_chat::p2p_chat().await;
    // let _ = p2p_gossipsub::gossipsub_chat().await;
    // let _ = ipfs_kad::ipfs_kad().await;
    let _ = substrate_node::connect_node().await;
}

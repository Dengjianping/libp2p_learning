mod p2p_ping;
mod p2p_chat;

#[tokio::main]
async fn main() {
    // let _ = p2p_ping::p2p_ping().await;
    let _ = p2p_chat::p2p_chat().await;
}

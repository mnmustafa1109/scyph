//! Helpers shared by the `s3_*` test binaries. Each `tests/*.rs` file is its own process,
//! so anything both need lives here.

use std::net::SocketAddr;

/// Binds a loopback port and accepts connections forever without ever reading or writing,
/// so the client's request hangs — what a paused MinIO, a broken proxy or a misrouted load
/// balancer looks like. The sockets are held open on purpose: dropping them would close the
/// connection and produce a fast error instead of a hang.
pub async fn spawn_hung_server() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((sock, _)) = listener.accept().await {
            held.push(sock);
        }
    });
    addr
}

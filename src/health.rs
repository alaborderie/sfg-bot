use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::sleep;

const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 3\r\nConnection: close\r\n\r\nok\n";
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(100);

/// Spawns a minimal HTTP/TCP health probe listener on `0.0.0.0:port`.
///
/// Any incoming connection receives a `200 OK` and is closed. This makes the
/// listener compatible with both Kubernetes `tcpSocket` and `httpGet` probes
/// without pulling in a full HTTP framework.
///
/// The response is written inline in the accept loop (it's ~80 bytes) so an
/// unauthenticated peer cannot fan out unbounded tasks. On accept errors the
/// loop backs off briefly to avoid hot-looping on persistent failures like FD
/// exhaustion.
pub fn spawn(port: u16) {
    tokio::spawn(async move {
        let listener = match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(error = %e, port, "Failed to bind health check listener");
                return;
            }
        };
        tracing::info!(port, "Health check listener started");
        loop {
            match listener.accept().await {
                Ok((mut stream, _addr)) => {
                    if let Err(e) = stream.write_all(RESPONSE).await {
                        tracing::debug!(error = %e, "Failed to write health response");
                    }
                    let _ = stream.shutdown().await;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Health check accept failed; backing off");
                    sleep(ACCEPT_ERROR_BACKOFF).await;
                }
            }
        }
    });
}

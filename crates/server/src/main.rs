//! Battler's multiplayer server.
//!
//! For now this is the skeleton that proves the hosting works end to end:
//!
//! - `GET /health` answers `ok`. Render uses it to check the service is up, and
//!   the web build will call it on page load to wake the server from sleep.
//! - `GET /ws` upgrades to a WebSocket that greets the client and echoes back
//!   whatever it sends.
//!
//! Rooms, the authoritative lobby and the command log come next (see the
//! Multiplayer section of `docs/DESIGN.md`).

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;

/// Sent to every new connection, so a client can tell it reached the right server.
const GREETING: &str = concat!("battler-server ", env!("CARGO_PKG_VERSION"));

// `#[tokio::main]` turns `async fn main` into a normal `fn main` that starts
// tokio's runtime and blocks on this future until it finishes.
#[tokio::main]
async fn main() {
    // Render tells the service which port to listen on through `PORT`. Bind to
    // 0.0.0.0 (every interface), not 127.0.0.1, or outside traffic can't reach us.
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(10000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let app = Router::new()
        .route("/", get(|| async { GREETING }))
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_upgrade));

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind the port");
    println!("{GREETING} listening on {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

/// Accepts the HTTP request that asks to become a WebSocket. The actual
/// connection is handled by `connection` once the upgrade completes.
async fn ws_upgrade(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(connection)
}

/// One client's connection: greet, then echo until it closes.
async fn connection(mut socket: WebSocket) {
    if socket.send(Message::Text(GREETING.into())).await.is_err() {
        return;
    }
    // `recv` yields `None` when the client goes away. `while let` stops there,
    // and also on the first error, since `Some(Ok(..))` no longer matches.
    while let Some(Ok(message)) = socket.recv().await {
        let reply = match message {
            Message::Text(text) => Message::Text(text),
            Message::Binary(bytes) => Message::Binary(bytes),
            // axum answers pings by itself; nothing else needs a reply.
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
        };
        if socket.send(reply).await.is_err() {
            break;
        }
    }
}

/// Resolves when the process is asked to stop. Render sends SIGTERM before a
/// redeploy; finishing cleanly lets open connections close properly.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("listen for ctrl-c");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    // `select!` waits for whichever future finishes first.
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

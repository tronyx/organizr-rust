//! The connections of the server: each one is served over HTTP/1.1 until it closes.
//!
//! `axum::serve` sets no time limit for the headers of a request, so a client that
//! sends them very slowly holds its connection open for good. Here each connection has
//! [`HEADER_TIME`] for the headers of each request. Nothing limits the time of a whole
//! request: the streams of the chat, backups and uploads take long.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::pin;
use std::time::Duration;

use axum::Router;
use axum::extract::{ConnectInfo, Request};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tower::ServiceExt as _;

/// How long a client may take to send the headers of a request.
pub const HEADER_TIME: Duration = Duration::from_secs(30);

/// Serves the connections of a listener until `stop` completes, and then waits until the
/// open connections have closed. The application finds the address of each connection
/// as [`ConnectInfo`], as with `axum::serve`.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    header_time: Duration,
    stop: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    // Dropping `stopping` asks each connection to finish its request and close.
    let (stopping, stop_seen) = watch::channel(());
    // Each connection holds a `closed`, so `all_closed` knows when the last one ended.
    let (all_closed, closed) = watch::channel(());
    let mut stop = pin!(stop);
    loop {
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            () = &mut stop => break,
        };
        let (stream, peer) = match accepted {
            Ok(accepted) => accepted,
            Err(error) => {
                wait_after(&error).await;
                continue;
            }
        };
        tokio::spawn(connection(
            stream,
            peer,
            app.clone(),
            header_time,
            (stop_seen.clone(), closed.clone()),
        ));
    }
    drop(listener);
    drop(stopping);
    drop(closed);
    all_closed.closed().await;
    Ok(())
}

/// What a connection gets from the server: the sign to stop, and the mark that it is
/// still open.
type Signals = (watch::Receiver<()>, watch::Receiver<()>);

/// Serves one connection until it closes, or until the server stops.
async fn connection(
    stream: TcpStream,
    peer: SocketAddr,
    app: Router,
    header_time: Duration,
    (mut stop_seen, open): Signals,
) {
    let service = hyper::service::service_fn(move |mut request: Request<Incoming>| {
        request.extensions_mut().insert(ConnectInfo(peer));
        app.clone().oneshot(request)
    });
    let mut builder = http1::Builder::new();
    builder
        .timer(TokioTimer::new())
        .header_read_timeout(header_time);
    let mut served = pin!(
        builder
            .serve_connection(TokioIo::new(stream), service)
            .with_upgrades()
    );
    let mut stopping = false;
    loop {
        tokio::select! {
            result = served.as_mut() => {
                if let Err(error) = result {
                    tracing::trace!(%error, "a connection ended with an error");
                }
                break;
            }
            _ = stop_seen.changed(), if !stopping => {
                stopping = true;
                served.as_mut().graceful_shutdown();
            }
        }
    }
    drop(open);
}

/// Waits after a failed accept as `axum::serve` does: a connection that broke before it
/// was accepted costs nothing, and any other error, such as too many open files, waits a
/// second so that the loop does not spin.
async fn wait_after(error: &io::Error) {
    if matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    ) {
        return;
    }
    tracing::error!(%error, "a connection could not be accepted");
    tokio::time::sleep(Duration::from_secs(1)).await;
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use axum::Router;
    use axum::extract::ConnectInfo;
    use axum::routing::{get, post};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;

    use super::serve;

    /// A server on a free local port, whose one route answers with the address of the
    /// caller. Sending on the channel stops it.
    async fn server(header_time: Duration) -> (SocketAddr, oneshot::Sender<()>) {
        let app =
            Router::new()
                .route(
                    "/",
                    get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move {
                        peer.ip().to_string()
                    }),
                )
                .route(
                    "/body",
                    post(|body: String| async move { body.len().to_string() }),
                );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let address = listener.local_addr().expect("the port has an address");
        let (stop, stopped) = oneshot::channel::<()>();
        tokio::spawn(serve(listener, app, header_time, async move {
            let _ = stopped.await;
        }));
        (address, stop)
    }

    /// Everything that the server sends until it closes the connection, or `None` when it
    /// keeps the connection open longer than `wait`.
    async fn answer(stream: &mut TcpStream, wait: Duration) -> Option<String> {
        let mut read = Vec::new();
        tokio::time::timeout(wait, stream.read_to_end(&mut read))
            .await
            .ok()?
            .ok()?;
        Some(String::from_utf8_lossy(&read).into_owned())
    }

    #[tokio::test]
    async fn request_is_answered_with_the_address_of_the_connection() {
        let (address, stop) = server(Duration::from_secs(5)).await;
        let mut stream = TcpStream::connect(address).await.expect("it connects");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: organizr\r\nConnection: close\r\n\r\n")
            .await
            .expect("it sends");
        let text = answer(&mut stream, Duration::from_secs(5))
            .await
            .expect("the server answers and closes");
        assert!(text.starts_with("HTTP/1.1 200"), "{text}");
        assert!(text.ends_with("127.0.0.1"), "{text}");
        let _ = stop.send(());
    }

    #[tokio::test]
    async fn client_that_is_slow_with_its_headers_is_cut_off() {
        let (address, stop) = server(Duration::from_millis(300)).await;
        let mut stream = TcpStream::connect(address).await.expect("it connects");
        // The headers never end.
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: organizr\r\n")
            .await
            .expect("it sends");
        assert!(
            answer(&mut stream, Duration::from_secs(5)).await.is_some(),
            "the server keeps a connection open whose headers do not end"
        );
        let _ = stop.send(());
    }

    #[tokio::test]
    async fn slow_body_is_no_slow_header() {
        // Only the headers have a time limit: an upload may take long.
        let (address, stop) = server(Duration::from_millis(300)).await;
        let mut stream = TcpStream::connect(address).await.expect("it connects");
        stream
            .write_all(
                b"POST /body HTTP/1.1\r\nHost: organizr\r\nContent-Length: 4\r\n\
                  Connection: close\r\n\r\nab",
            )
            .await
            .expect("it sends");
        tokio::time::sleep(Duration::from_millis(800)).await;
        stream.write_all(b"cd").await.expect("it sends the rest");
        let text = answer(&mut stream, Duration::from_secs(5))
            .await
            .expect("the server answers and closes");
        assert!(text.starts_with("HTTP/1.1 200"), "{text}");
        assert!(text.ends_with('4'), "{text}");
        let _ = stop.send(());
    }

    #[tokio::test]
    async fn server_stops_when_asked() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let ended = tokio::time::timeout(
            Duration::from_secs(5),
            serve(listener, Router::new(), Duration::from_secs(5), async {}),
        )
        .await;
        assert!(matches!(ended, Ok(Ok(()))), "{ended:?}");
    }
}

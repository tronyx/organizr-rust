//! The connections of the server: each one is served over HTTP/1.1 until it closes.
//!
//! `axum::serve` sets no time limit for the headers of a request, so a client that
//! sends them very slowly holds its connection open for good. Here each connection has
//! [`Limits::header_time`] for the headers of each request, and a request body may pause
//! for at most [`Limits::body_pause`] between two of its parts. Nothing limits the time
//! of a whole request: the streams of the chat, backups and uploads take long, and an
//! upload keeps sending. At most [`Limits::most_connections`] connections are open at
//! once; a further client waits until one closes, so the process never runs out of open
//! files.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{ConnectInfo, Request};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tower::ServiceExt as _;
use tower_http::timeout::TimeoutBody;

/// How long a client may take to send the headers of a request.
pub const HEADER_TIME: Duration = Duration::from_secs(30);

/// How long a request body may pause between two of its parts.
pub const BODY_PAUSE: Duration = Duration::from_mins(1);

/// The most connections that are open at once, when the limit of open files of the
/// process allows them.
pub const MOST_CONNECTIONS: usize = 4096;

/// The limits of the connections of a server.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// How long a client may take to send the headers of a request.
    pub header_time: Duration,
    /// How long a request body may pause between two of its parts.
    pub body_pause: Duration,
    /// How many connections are open at once.
    pub most_connections: usize,
}

impl Limits {
    /// The limits of the server: [`HEADER_TIME`], [`BODY_PAUSE`], and
    /// [`MOST_CONNECTIONS`], or three quarters of the limit of open files of the process
    /// when that is lower, so that the database, the files and the requests to services
    /// keep some.
    pub fn of_this_process() -> Self {
        let most_connections = open_file_limit()
            .map_or(MOST_CONNECTIONS, |limit| {
                (limit / 4 * 3).min(MOST_CONNECTIONS)
            })
            .max(16);
        Self {
            header_time: HEADER_TIME,
            body_pause: BODY_PAUSE,
            most_connections,
        }
    }
}

/// The soft limit of open files of the process, from `/proc/self/limits` on Linux, or
/// `None` where it cannot be read or is unlimited.
fn open_file_limit() -> Option<usize> {
    let limits = std::fs::read_to_string("/proc/self/limits").ok()?;
    soft_open_file_limit(&limits)
}

/// The soft limit of the line "Max open files" of a text of `/proc/self/limits`.
fn soft_open_file_limit(limits: &str) -> Option<usize> {
    limits
        .lines()
        .find_map(|line| line.strip_prefix("Max open files"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|soft| soft.parse().ok())
}

/// Serves the connections of a listener until `stop` completes, and then waits until the
/// open connections have closed. The application finds the address of each connection
/// as [`ConnectInfo`], as with `axum::serve`.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    limits: Limits,
    stop: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    // Dropping `stopping` asks each connection to finish its request and close.
    let (stopping, stop_seen) = watch::channel(());
    // Each connection holds a `closed`, so `all_closed` knows when the last one ended.
    let (all_closed, closed) = watch::channel(());
    // Each connection holds a place; with none left, the next client waits in the queue
    // of the listener until a connection closes.
    let places = Arc::new(Semaphore::new(limits.most_connections));
    let mut stop = pin!(stop);
    loop {
        let place = tokio::select! {
            place = Arc::clone(&places).acquire_owned() => place,
            () = &mut stop => break,
        };
        let Ok(place) = place else { break };
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
            limits,
            (stop_seen.clone(), closed.clone(), place),
        ));
    }
    drop(listener);
    drop(stopping);
    drop(closed);
    all_closed.closed().await;
    Ok(())
}

/// What a connection gets from the server: the sign to stop, the mark that it is still
/// open, and its place among the open connections.
type Signals = (
    watch::Receiver<()>,
    watch::Receiver<()>,
    OwnedSemaphorePermit,
);

/// Serves one connection until it closes, or until the server stops.
async fn connection(
    stream: TcpStream,
    peer: SocketAddr,
    app: Router,
    limits: Limits,
    (mut stop_seen, open, place): Signals,
) {
    let service = hyper::service::service_fn(move |request: Request<Incoming>| {
        // A body that pauses too long ends with an error, and so does its request.
        let mut request = request.map(|body| TimeoutBody::new(limits.body_pause, body));
        request.extensions_mut().insert(ConnectInfo(peer));
        app.clone().oneshot(request)
    });
    let mut builder = http1::Builder::new();
    builder
        .timer(TokioTimer::new())
        .header_read_timeout(limits.header_time);
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
    drop(place);
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

    use super::{Limits, serve, soft_open_file_limit};

    /// Limits with the given time for the headers, for the tests.
    fn limits(header_time: Duration) -> Limits {
        Limits {
            header_time,
            body_pause: Duration::from_secs(5),
            most_connections: 64,
        }
    }

    /// A server on a free local port, whose one route answers with the address of the
    /// caller. Sending on the channel stops it.
    async fn server(header_time: Duration) -> (SocketAddr, oneshot::Sender<()>) {
        server_with(limits(header_time)).await
    }

    async fn server_with(limits: Limits) -> (SocketAddr, oneshot::Sender<()>) {
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
        tokio::spawn(serve(listener, app, limits, async move {
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
    async fn body_that_pauses_too_long_is_cut_off() {
        let (address, stop) = server_with(Limits {
            body_pause: Duration::from_millis(300),
            ..limits(Duration::from_secs(5))
        })
        .await;
        let mut stream = TcpStream::connect(address).await.expect("it connects");
        // The headers announce a body, which never comes.
        stream
            .write_all(
                b"POST /body HTTP/1.1\r\nHost: organizr\r\nContent-Length: 100000\r\n\
                  Connection: close\r\n\r\n",
            )
            .await
            .expect("it sends");
        let text = answer(&mut stream, Duration::from_secs(5))
            .await
            .expect("the server gives up on the body and closes");
        assert!(text.starts_with("HTTP/1.1 4"), "{text}");
        let _ = stop.send(());
    }

    #[tokio::test]
    async fn client_above_the_most_connections_waits_for_a_place() {
        let (address, stop) = server_with(Limits {
            most_connections: 2,
            ..limits(Duration::from_secs(5))
        })
        .await;
        let first = TcpStream::connect(address).await.expect("it connects");
        let _second = TcpStream::connect(address).await.expect("it connects");
        let mut third = TcpStream::connect(address).await.expect("it connects");
        third
            .write_all(b"GET / HTTP/1.1\r\nHost: organizr\r\nConnection: close\r\n\r\n")
            .await
            .expect("it sends");
        assert!(
            answer(&mut third, Duration::from_millis(500))
                .await
                .is_none(),
            "a third connection was served while two were open"
        );
        drop(first);
        let text = answer(&mut third, Duration::from_secs(5))
            .await
            .expect("the third connection is served once a place is free");
        assert!(text.starts_with("HTTP/1.1 200"), "{text}");
        let _ = stop.send(());
    }

    #[test]
    fn soft_limit_of_open_files_is_read() {
        let limits = "Limit                     Soft Limit           Hard Limit           Units     \n\
                      Max processes             63432                63432                processes \n\
                      Max open files            1024                 524288               files     \n";
        assert_eq!(soft_open_file_limit(limits), Some(1024));
        let unlimited =
            "Max open files            unlimited            unlimited            files\n";
        assert_eq!(soft_open_file_limit(unlimited), None);
    }

    #[tokio::test]
    async fn server_stops_when_asked() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let ended = tokio::time::timeout(
            Duration::from_secs(5),
            serve(
                listener,
                Router::new(),
                limits(Duration::from_secs(5)),
                async {},
            ),
        )
        .await;
        assert!(matches!(ended, Ok(Ok(()))), "{ended:?}");
    }
}

//! A certificate authority of the tests, a certificate for `localhost` that it signed,
//! and what serves a stand-in over HTTPS with that certificate.
//!
//! The files were made once with OpenSSL. The key of the authority was not kept. The
//! key of the certificate for `localhost` is in the tests on purpose: it serves only
//! the stand-ins of the tests, and no system trusts the authority.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

/// The certificate of the authority of the tests, as PEM text.
pub const AUTHORITY: &str = include_str!("../fixtures/tls/test-authority.pem");

/// The certificate for the name `localhost`, which the authority signed. It is no
/// certificate of an authority.
pub const LOCALHOST_CERTIFICATE: &str = include_str!("../fixtures/tls/localhost.pem");

/// The private key of [`LOCALHOST_CERTIFICATE`], as PEM text.
pub const LOCALHOST_KEY: &str = include_str!("../fixtures/tls/localhost.key");

/// A request for a certificate, as PEM text: a PEM part that is no certificate.
pub const CERTIFICATE_REQUEST: &str = include_str!("../fixtures/tls/localhost.csr");

/// What makes the TLS connections of a stand-in.
pub fn acceptor() -> TlsAcceptor {
    let certificates = CertificateDer::pem_slice_iter(LOCALHOST_CERTIFICATE.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .expect("the certificate of the tests can be read");
    let key = PrivateKeyDer::from_pem_slice(LOCALHOST_KEY.as_bytes())
        .expect("the key of the tests can be read");
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("the versions of TLS are supported")
    .with_no_client_auth()
    .with_single_cert(certificates, key)
    .expect("the certificate fits the key");
    TlsAcceptor::from(Arc::new(config))
}

/// A listener that makes a TLS connection of each connection that it takes. A
/// connection whose client gives up during the handshake is left out.
pub struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl TlsListener {
    /// The listener around a TCP listener.
    pub const fn new(listener: TcpListener, acceptor: TlsAcceptor) -> Self {
        Self { listener, acceptor }
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let Ok((stream, address)) = self.listener.accept().await else {
                continue;
            };
            if let Ok(tls) = self.acceptor.accept(stream).await {
                return (tls, address);
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

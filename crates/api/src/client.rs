//! Who sent a request: the address of the client, and whether the browser used HTTPS.
//!
//! The application usually runs behind a reverse proxy, so the connection comes from the
//! proxy and the address of the browser is in the `X-Forwarded-For` header. Anyone can
//! send that header. It is therefore believed only when the connection comes from a proxy
//! that the administrator has listed as trusted. The PHP application believed it always,
//! which let a caller claim any address.
//!
//! The same rule applies to `X-Forwarded-Proto`, which says whether the browser used
//! HTTPS.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use ipnet::IpNet;
use thiserror::Error;

use crate::state::AppState;

const FORWARDED_FOR: &str = "x-forwarded-for";
const FORWARDED_PROTO: &str = "x-forwarded-proto";

/// The reverse proxies whose forwarding headers are believed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustedProxies(Vec<IpNet>);

/// An entry of a trusted-proxy list that is neither an address nor a network.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{0}` is not an IP address or a network such as 10.0.0.0/8")]
pub struct InvalidProxy(String);

impl FromStr for TrustedProxies {
    type Err = InvalidProxy;

    /// Reads a list of addresses and networks separated by commas, for example
    /// `172.18.0.2, 10.0.0.0/8, fd00::/8`. An empty text trusts no proxy.
    fn from_str(list: &str) -> Result<Self, Self::Err> {
        list.split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                entry
                    .parse::<IpNet>()
                    .or_else(|_| entry.parse::<IpAddr>().map(IpNet::from))
                    .map_err(|_| InvalidProxy(entry.to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}

impl TrustedProxies {
    /// The list as text, with commas between the entries.
    pub fn text(&self) -> String {
        self.0
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Whether the list names no proxy.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether an address belongs to a trusted proxy.
    pub fn contains(&self, address: IpAddr) -> bool {
        self.0.iter().any(|network| network.contains(&address))
    }
}

/// The networks whose callers count as local: on the network of the server itself.
///
/// The PHP application uses this for the local address of a tab, among other things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNetworks(Vec<IpNet>);

impl LocalNetworks {
    /// The networks that are always local: the private ranges of IPv4, and the machine
    /// itself.
    const ALWAYS: [&str; 5] = [
        "10.0.0.0/8",
        "172.16.0.0/12",
        "192.168.0.0/16",
        "127.0.0.0/8",
        "::1/128",
    ];

    /// The networks that are always local, and those of the `localIPList` setting: a
    /// list of addresses and networks separated by commas, for example
    /// `203.0.113.7, 198.51.100.0/24`. An entry that is neither is left out. An entry
    /// such as `192.168.1.7/24` stands for its network.
    pub fn from_setting(list: &str) -> Self {
        let networks = Self::ALWAYS
            .into_iter()
            .chain(list.split(','))
            .map(str::trim)
            .filter_map(|entry| {
                entry
                    .parse::<IpNet>()
                    .or_else(|_| entry.parse::<IpAddr>().map(IpNet::from))
                    .ok()
            })
            .collect();
        Self(networks)
    }

    /// Whether a client is on a local network.
    ///
    /// A client behind a proxy that is not listed as trusted is never local: its true
    /// address is not known, and the address of the proxy itself is usually a local one.
    pub fn contains(&self, client: &Client) -> bool {
        client.source != AddressSource::UnverifiedForwarding
            && self
                .0
                .iter()
                .any(|network| network.contains(&client.address))
    }
}

/// How the address of a client is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSource {
    /// The address of the connection itself. No proxy is involved.
    Connection,
    /// A trusted proxy reported the address.
    TrustedProxy,
    /// The request carries a forwarding header, but the connection does not come from a
    /// trusted proxy. The address is that of the connection, and the true client is not
    /// known. Such a client must never get the rights of a local caller: when a proxy is
    /// in front but not listed as trusted, every caller would otherwise look local.
    UnverifiedForwarding,
}

/// The sender of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Client {
    /// The address of the client.
    pub address: IpAddr,
    /// How the address is known.
    pub source: AddressSource,
    /// Whether the browser reached the application over HTTPS, as a trusted proxy reports
    /// it. The application itself speaks plain HTTP.
    pub https: bool,
    /// Whether the connection comes from a trusted proxy. Only then are the other
    /// forwarding headers of the request believed.
    pub via_trusted_proxy: bool,
}

/// Works out the sender of a request from the address of the connection and the
/// forwarding headers.
///
/// From a trusted proxy, `X-Forwarded-For` is read from the right: each proxy appends the
/// address it received the request from, so the last address that is not itself a trusted
/// proxy is the client. Addresses further left were written by the client and prove
/// nothing.
pub fn resolve(peer: IpAddr, headers: &HeaderMap, trusted: &TrustedProxies) -> Client {
    let peer = peer.to_canonical();

    if !trusted.contains(peer) {
        let source = if headers.contains_key(FORWARDED_FOR) {
            AddressSource::UnverifiedForwarding
        } else {
            AddressSource::Connection
        };
        return Client {
            address: peer,
            source,
            https: false,
            via_trusted_proxy: false,
        };
    }

    let forwarded: Vec<&str> = headers
        .get_all(FORWARDED_FOR)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .collect();

    let mut address = peer;
    let mut source = AddressSource::Connection;
    for entry in forwarded.into_iter().rev() {
        let Some(reported) = forwarded_address(entry) else {
            break;
        };
        address = reported;
        source = AddressSource::TrustedProxy;
        if !trusted.contains(reported) {
            break;
        }
    }

    let https = headers
        .get(FORWARDED_PROTO)
        .is_some_and(|value| value.as_bytes() == b"https");
    Client {
        address,
        source,
        https,
        via_trusted_proxy: true,
    }
}

/// Reads one entry of `X-Forwarded-For`: an address, or an address with a port.
fn forwarded_address(entry: &str) -> Option<IpAddr> {
    let entry = entry.trim();
    entry
        .parse::<IpAddr>()
        .or_else(|_| entry.parse::<SocketAddr>().map(|socket| socket.ip()))
        .ok()
        .map(|address| address.to_canonical())
}

/// Works out the sender of each request and makes it available to the handlers as a
/// [`Client`] in the request extensions. Use it as a middleware.
pub async fn identify_client(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // A request that did not arrive over a connection (a test, for example) has no peer.
    // The unspecified address belongs to no network, so it is never trusted or local.
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |info| info.0.ip());

    let client = resolve(peer, request.headers(), &state.trusted_proxies());
    if client.source == AddressSource::UnverifiedForwarding {
        warn_once_about_unlisted_proxy(client.address);
    }

    request.extensions_mut().insert(client);
    next.run(request).await
}

/// Tells the administrator, once, that a proxy seems to be in front without being listed.
fn warn_once_about_unlisted_proxy(peer: IpAddr) {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        tracing::warn!(
            %peer,
            "a request with an X-Forwarded-For header came from an address that is not a \
             trusted proxy, so the header was ignored. If this address is your reverse \
             proxy, add it to ORGANIZR_TRUSTED_PROXIES."
        );
    }
}

#[cfg(test)]
mod local_network_tests {
    use std::net::IpAddr;

    use super::{AddressSource, Client, LocalNetworks};

    fn client(address: &str, source: AddressSource) -> Client {
        Client {
            address: address.parse::<IpAddr>().expect("the address is valid"),
            source,
            https: false,
            via_trusted_proxy: false,
        }
    }

    fn is_local(list: &str, address: &str) -> bool {
        LocalNetworks::from_setting(list).contains(&client(address, AddressSource::Connection))
    }

    #[test]
    fn private_and_loopback_addresses_are_always_local() {
        for address in [
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.1",
            "192.168.255.255",
            "127.0.0.1",
            "127.255.255.255",
            "::1",
        ] {
            assert!(is_local("", address), "{address}");
        }
    }

    #[test]
    fn other_addresses_are_not_local() {
        for address in [
            "9.255.255.255",
            "11.0.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.167.255.255",
            "192.169.0.0",
            "203.0.113.7",
            "0.0.0.0",
            "::",
            "fd00::1",
            "2001:db8::1",
        ] {
            assert!(!is_local("", address), "{address}");
        }
    }

    #[test]
    fn setting_adds_addresses_and_networks() {
        let list =
            " 203.0.113.7 , 198.51.100.0/24,fd00::/8, not-an-address, 300.1.1.1,, 192.0.2.99/28";

        for address in [
            "203.0.113.7",
            "198.51.100.0",
            "198.51.100.255",
            "fd00::1",
            // 192.0.2.99/28 stands for the network 192.0.2.96/28.
            "192.0.2.96",
            "192.0.2.111",
            // The networks that are always local stay local.
            "10.0.0.1",
        ] {
            assert!(is_local(list, address), "{address}");
        }
        for address in ["203.0.113.8", "198.51.101.0", "192.0.2.95", "192.0.2.112"] {
            assert!(!is_local(list, address), "{address}");
        }
    }

    #[test]
    fn client_behind_a_proxy_that_is_not_trusted_is_never_local() {
        let networks = LocalNetworks::from_setting("");

        assert!(networks.contains(&client("192.168.1.5", AddressSource::Connection)));
        assert!(networks.contains(&client("192.168.1.5", AddressSource::TrustedProxy)));
        assert!(!networks.contains(&client("192.168.1.5", AddressSource::UnverifiedForwarding)));
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use axum::http::{HeaderMap, HeaderName, HeaderValue};

    use super::{AddressSource, Client, InvalidProxy, TrustedProxies, resolve};

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("the address is valid")
    }

    fn proxies(list: &str) -> TrustedProxies {
        list.parse().expect("the list is valid")
    }

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            );
        }
        headers
    }

    /// The address and its source for a request from `peer` with the given
    /// `X-Forwarded-For` header, when `trusted` lists the trusted proxies.
    fn sender(
        peer: &str,
        forwarded_for: Option<&'static str>,
        trusted: &str,
    ) -> (IpAddr, AddressSource) {
        let headers = forwarded_for.map_or_else(HeaderMap::new, |value| {
            headers(&[("x-forwarded-for", value)])
        });
        let client = resolve(ip(peer), &headers, &proxies(trusted));
        (client.address, client.source)
    }

    #[test]
    fn list_accepts_addresses_and_networks_of_both_families() {
        let trusted = proxies(" 172.18.0.2 ,10.0.0.0/8, fd00::/8,::1 ");

        assert!(trusted.contains(ip("172.18.0.2")));
        assert!(!trusted.contains(ip("172.18.0.3")));
        assert!(trusted.contains(ip("10.200.3.4")));
        assert!(trusted.contains(ip("fd12:3456::1")));
        assert!(trusted.contains(ip("::1")));
        assert!(!trusted.contains(ip("203.0.113.7")));
    }

    #[test]
    fn empty_list_trusts_no_proxy() {
        for list in ["", "  ", ","] {
            let trusted = proxies(list);
            assert_eq!(trusted, TrustedProxies::default(), "list `{list}`");
            assert!(!trusted.contains(ip("127.0.0.1")));
        }
    }

    #[test]
    fn entry_that_is_not_an_address_is_refused() {
        assert_eq!(
            "10.0.0.1, proxy.local".parse::<TrustedProxies>(),
            Err(InvalidProxy("proxy.local".to_owned()))
        );
        assert_eq!(
            "10.0.0.0/40".parse::<TrustedProxies>(),
            Err(InvalidProxy("10.0.0.0/40".to_owned()))
        );
    }

    #[test]
    fn direct_caller_is_known_by_its_connection() {
        assert_eq!(
            sender("203.0.113.7", None, ""),
            (ip("203.0.113.7"), AddressSource::Connection)
        );
    }

    #[test]
    fn header_from_an_unlisted_address_is_ignored_and_flagged() {
        // The caller claims a local address. It is not believed.
        assert_eq!(
            sender("203.0.113.7", Some("192.168.1.10"), ""),
            (ip("203.0.113.7"), AddressSource::UnverifiedForwarding)
        );
        // The same when proxies are listed and this address is not one of them.
        assert_eq!(
            sender("203.0.113.7", Some("192.168.1.10"), "172.18.0.0/16"),
            (ip("203.0.113.7"), AddressSource::UnverifiedForwarding)
        );
    }

    #[test]
    fn trusted_proxy_reports_the_client() {
        assert_eq!(
            sender("172.18.0.2", Some("203.0.113.7"), "172.18.0.0/16"),
            (ip("203.0.113.7"), AddressSource::TrustedProxy)
        );
    }

    #[test]
    fn address_that_the_client_wrote_itself_is_not_believed() {
        // The client sent "X-Forwarded-For: 192.168.1.10"; the proxy appended the address
        // it really saw.
        assert_eq!(
            sender(
                "172.18.0.2",
                Some("192.168.1.10, 203.0.113.7"),
                "172.18.0.0/16"
            ),
            (ip("203.0.113.7"), AddressSource::TrustedProxy)
        );
    }

    #[test]
    fn chain_of_trusted_proxies_is_followed_to_the_client() {
        assert_eq!(
            sender(
                "172.18.0.2",
                Some("203.0.113.7, 10.0.0.5"),
                "172.18.0.0/16, 10.0.0.0/8"
            ),
            (ip("203.0.113.7"), AddressSource::TrustedProxy)
        );
    }

    #[test]
    fn chain_of_only_trusted_proxies_gives_the_first_of_them() {
        assert_eq!(
            sender(
                "172.18.0.2",
                Some("10.0.0.9, 10.0.0.5"),
                "172.18.0.0/16, 10.0.0.0/8"
            ),
            (ip("10.0.0.9"), AddressSource::TrustedProxy)
        );
    }

    #[test]
    fn trusted_proxy_without_the_header_is_the_caller_itself() {
        assert_eq!(
            sender("172.18.0.2", None, "172.18.0.0/16"),
            (ip("172.18.0.2"), AddressSource::Connection)
        );
    }

    #[test]
    fn entry_with_a_port_is_read() {
        assert_eq!(
            sender("172.18.0.2", Some("203.0.113.7:51234"), "172.18.0.0/16").0,
            ip("203.0.113.7")
        );
        assert_eq!(
            sender("172.18.0.2", Some("[2001:db8::7]:51234"), "172.18.0.0/16").0,
            ip("2001:db8::7")
        );
    }

    #[test]
    fn reading_stops_at_an_entry_that_is_not_an_address() {
        // Nothing left of the broken entry is used.
        assert_eq!(
            sender(
                "172.18.0.2",
                Some("203.0.113.7, unknown, 10.0.0.5"),
                "172.18.0.0/16, 10.0.0.0/8"
            ),
            (ip("10.0.0.5"), AddressSource::TrustedProxy)
        );
        assert_eq!(
            sender("172.18.0.2", Some("unknown"), "172.18.0.0/16"),
            (ip("172.18.0.2"), AddressSource::Connection)
        );
    }

    #[test]
    fn several_header_lines_are_read_as_one_list() {
        let headers = headers(&[
            ("x-forwarded-for", "192.168.1.10"),
            ("x-forwarded-for", "203.0.113.7, 10.0.0.5"),
        ]);

        let client = resolve(
            ip("172.18.0.2"),
            &headers,
            &proxies("172.18.0.0/16, 10.0.0.0/8"),
        );

        assert_eq!(client.address, ip("203.0.113.7"));
    }

    #[test]
    fn ipv4_peer_on_an_ipv6_socket_is_seen_as_ipv4() {
        assert_eq!(
            sender("::ffff:172.18.0.2", Some("203.0.113.7"), "172.18.0.0/16"),
            (ip("203.0.113.7"), AddressSource::TrustedProxy)
        );
        assert_eq!(sender("::ffff:203.0.113.7", None, "").0, ip("203.0.113.7"));
    }

    #[test]
    fn https_is_believed_only_from_a_trusted_proxy() {
        let request = headers(&[
            ("x-forwarded-for", "203.0.113.7"),
            ("x-forwarded-proto", "https"),
        ]);

        let through_proxy = resolve(ip("172.18.0.2"), &request, &proxies("172.18.0.0/16"));
        let direct = resolve(ip("203.0.113.9"), &request, &proxies("172.18.0.0/16"));

        assert!(through_proxy.https);
        assert_eq!(
            direct,
            Client {
                address: ip("203.0.113.9"),
                source: AddressSource::UnverifiedForwarding,
                https: false,
                via_trusted_proxy: false,
            }
        );
    }

    #[test]
    fn only_the_exact_value_means_https() {
        for value in ["http", "HTTPS", "https, http"] {
            let request = headers(&[("x-forwarded-proto", value)]);
            let client = resolve(ip("172.18.0.2"), &request, &proxies("172.18.0.0/16"));
            assert!(!client.https, "value `{value}`");
        }
    }
}

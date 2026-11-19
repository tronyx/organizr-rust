//! Clients for the services that Organizr talks to: the account service of Plex, the
//! applications that take part in single sign-on, and the applications whose data the
//! homepage shows.

pub mod arr;
pub mod downloaders;
pub mod health_checks;
pub mod http;
pub mod overseerr;
pub mod plex;
pub mod plex_server;
pub mod sso;
pub mod tautulli;
pub mod uptime_kuma;
pub mod weather;

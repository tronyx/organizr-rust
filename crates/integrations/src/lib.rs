//! Clients for the services that Organizr talks to: the account service of Plex, the
//! applications that take part in single sign-on, and the applications whose data the
//! homepage shows.

pub mod adguard;
pub mod arr;
pub mod couchpotato;
pub mod downloaders;
pub mod downloaders_more;
pub mod emby;
pub mod health_checks;
pub mod http;
pub mod ical;
pub mod indexers;
pub mod monitorr;
pub mod netdata;
pub mod octoprint;
pub mod ombi;
pub mod overseerr;
pub mod pihole;
pub mod plex;
pub mod plex_server;
pub mod probe;
pub mod sickrage;
pub mod speedtest_tracker;
pub mod sso;
pub mod tautulli;
pub mod unifi;
pub mod uptime_kuma;
pub mod weather;

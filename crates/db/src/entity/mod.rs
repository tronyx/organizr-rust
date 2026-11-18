//! Table rows as the database stores them.
//!
//! These types use plain column types and stay inside this crate. The repositories turn
//! them into domain types and check the values on the way, so the rest of the application
//! never sees a row that breaks a domain rule.

#![allow(
    clippy::unused_async_trait_impl,
    reason = "the code that `sea_orm::model` generates has async functions without an await"
)]

pub mod group;
pub mod user;

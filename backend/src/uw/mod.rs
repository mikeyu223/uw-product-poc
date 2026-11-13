//! Unusual Whales API client.
//!
//! Base URL: `https://api.unusualwhales.com`
//! Auth: `Authorization: Bearer <UNUSUAL_WHALES_API_KEY>`

mod client;
mod error;
mod models;

pub use client::UwClient;
pub use error::{UwError, UwResult};
pub use models::*;

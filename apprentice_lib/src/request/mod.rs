//! API transport.
mod client;
mod reqwest;

#[cfg(test)]
pub mod stub;

pub use client::Client;
pub use client::get_reqwest_client;
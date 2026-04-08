pub mod auth;
mod conn;
mod message_stream;
mod notification;
mod peer;

pub use conn::Connection;
pub use notification::*;
pub use peer::*;
pub use raijin_proto as proto;
pub use raijin_proto::{Receipt, TypedEnvelope, error::*};
mod macros;

#[cfg(feature = "inazuma")]
mod proto_client;
#[cfg(feature = "inazuma")]
pub use proto_client::*;

pub const PROTOCOL_VERSION: u32 = 68;

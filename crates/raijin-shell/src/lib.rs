mod context;
mod metadata;
pub mod shell_install;

pub use context::{shorten_path, GitStats, ShellContext};
pub use metadata::ShellMetadataPayload;

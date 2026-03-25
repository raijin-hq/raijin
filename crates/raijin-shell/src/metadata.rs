/// Shell metadata received via OSC 7777;raijin-precmd.
///
/// Deserialized from hex-encoded JSON sent by the shell precmd hook.
/// Contains environment context for updating UI chips and block headers.
///
/// Extensible with `#[serde(default)]` — new fields can be added without
/// breaking shells that don't send them yet.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ShellMetadataPayload {
    pub cwd: String,
    pub username: Option<String>,
    pub hostname: Option<String>,
    pub git_branch: Option<String>,
    pub git_dirty: Option<bool>,
    pub last_exit_code: Option<i32>,
    pub last_duration_ms: Option<u64>,
    pub shell: Option<String>,
}

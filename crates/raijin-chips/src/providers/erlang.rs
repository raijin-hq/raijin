use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Erlang/OTP version.
///
/// Detection: `rebar.config`, `erlang.mk`, `.erl`, `.hrl` files.
/// Version: Uses `erl -noshell -eval` to read the OTP_VERSION file,
///   which gives the full version like `26.2.1`. Falls back to `erl +V` which prints
///   to stderr: `Erlang (SES) 26.0` -> `26.0`.
///

pub struct ErlangProvider;

impl ChipProvider for ErlangProvider {
    fn id(&self) -> ChipId {
        "erlang"
    }

    fn display_name(&self) -> &str {
        "Erlang"
    }

    fn detect_files(&self) -> &[&str] {
        &["rebar.config", "erlang.mk"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["erl", "hrl"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_erlang_version(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Erlang"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Erlang/OTP {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Get Erlang/OTP version using the most reliable method first.
fn get_erlang_version(ctx: &ChipContext) -> Option<String> {
    // Primary: Read OTP_VERSION file via erl (standard pattern — gives full version like 26.2.1)
    if let Some(output) = ctx.exec_cmd(
        "erl",
        &[
            "-noshell",
            "-eval",
            r#"Fn=filename:join([code:root_dir(),"releases",erlang:system_info(otp_release),"OTP_VERSION"]),{ok,Content}=file:read_file(Fn),io:format("~s",[Content]),halt(0)."#,
        ],
    ) {
        let v = output.stdout.trim();
        if !v.is_empty() && v.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Some(v.to_string());
        }
    }

    // Fallback: `erl -eval` for OTP release (just major like "26")
    if let Some(output) = ctx.exec_cmd(
        "erl",
        &[
            "-noshell",
            "-eval",
            r#"io:format("~s",[erlang:system_info(otp_release)]),halt(0)."#,
        ],
    ) {
        let v = output.stdout.trim();
        if !v.is_empty() && v.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Some(v.to_string());
        }
    }

    // Last resort: `erl +V` (prints to stderr)
    if let Some(output) = ctx.exec_cmd("erl", &["+V"]) {
        let text = if output.stderr.trim().is_empty() {
            &output.stdout
        } else {
            &output.stderr
        };
        return parse_erl_version(text);
    }

    None
}

/// Parse Erlang version from `erl +V` stderr output.
///
/// Input: `Erlang (SES) 26.0\n` or `Erlang/OTP 26 [erts-14.2.1]`
/// Output: `Some("26.0")` or `Some("26")`
fn parse_erl_version(output: &str) -> Option<String> {
    for word in output.split_whitespace() {
        if word.chars().next().map_or(false, |c| c.is_ascii_digit())
            && word.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            return Some(word.to_string());
        }
    }
    None
}

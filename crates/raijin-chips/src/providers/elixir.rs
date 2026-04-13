use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Elixir runtime version.
///
/// Detection: `mix.exs` file; `ex`, `exs` extensions
/// Version:   `elixir --version` → parses Elixir version and OTP version
///
/// Output format: "1.16.0 (OTP 26)" matching standard elixir module.
pub struct ElixirProvider;

impl ChipProvider for ElixirProvider {
    fn id(&self) -> ChipId {
        "elixir"
    }

    fn display_name(&self) -> &str {
        "Elixir"
    }

    fn detect_files(&self) -> &[&str] {
        &["mix.exs"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["ex", "exs"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = ctx
            .exec_cmd("elixir", &["--version"])
            .and_then(|o| format_elixir_label(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Droplet"),
            tooltip: Some("Elixir version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `elixir --version` and format as "1.16.0 (OTP 26)".
///
/// The output looks like:
/// ```text
/// Erlang/OTP 26 [erts-14.2.1] [source] [64-bit] [smp:8:8] ...
///
/// Elixir 1.16.0 (compiled with Erlang/OTP 26)
/// ```
///
/// standard parses line 1 for OTP version (second token) and line 3 for
/// Elixir version (second token). We combine them into a single label.
fn format_elixir_label(output: &str) -> Option<String> {
    let (otp, elixir) = parse_elixir_version(output)?;
    Some(format!("{elixir} (OTP {otp})"))
}

/// Extract (otp_version, elixir_version) from `elixir --version` output.
fn parse_elixir_version(output: &str) -> Option<(String, String)> {
    let mut lines = output.lines();

    // Line 1: "Erlang/OTP 26 [erts-14.2.1] ..."
    let otp_version = lines.next()?.split_whitespace().nth(1)?;

    // Skip empty line
    let _ = lines.next()?;

    // Line 3: "Elixir 1.16.0 (compiled with Erlang/OTP 26)"
    let elixir_version = lines.next()?.split_whitespace().nth(1)?;

    Some((otp_version.to_string(), elixir_version.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stable() {
        let input = "\
Erlang/OTP 23 [erts-11.1.7] [source] [64-bit] [smp:4:4] [ds:4:4:10] [async-threads:1]

Elixir 1.11.3 (compiled with Erlang/OTP 21)
";
        assert_eq!(
            parse_elixir_version(input),
            Some(("23".to_string(), "1.11.3".to_string())),
        );
        assert_eq!(
            format_elixir_label(input),
            Some("1.11.3 (OTP 23)".to_string()),
        );
    }

    #[test]
    fn parse_rc() {
        let input = "\
Erlang/OTP 23 [erts-11.1.7] [source] [64-bit] [smp:4:4] [ds:4:4:10] [async-threads:1]

Elixir 1.12.0-rc.0 (31d2b99) (compiled with Erlang/OTP 21)
";
        assert_eq!(
            parse_elixir_version(input),
            Some(("23".to_string(), "1.12.0-rc.0".to_string())),
        );
    }

    #[test]
    fn parse_dev() {
        let input = "\
Erlang/OTP 23 [erts-11.1.7] [source] [64-bit] [smp:8:8] [ds:8:8:10] [async-threads:1]

Elixir 1.13.0-dev (compiled with Erlang/OTP 23)
";
        assert_eq!(
            parse_elixir_version(input),
            Some(("23".to_string(), "1.13.0-dev".to_string())),
        );
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse_elixir_version(""), None);
    }

    #[test]
    fn format_label() {
        let input = "\
Erlang/OTP 26 [erts-14.2.1] [source] [64-bit]

Elixir 1.16.0 (compiled with Erlang/OTP 26)
";
        assert_eq!(
            format_elixir_label(input),
            Some("1.16.0 (OTP 26)".to_string()),
        );
    }
}

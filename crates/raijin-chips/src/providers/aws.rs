
use std::cell::OnceCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::DateTime;
use ini::Ini;
use serde_json as json;
use sha1::{Digest, Sha1};

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

type Profile = String;
type Region = String;
type AwsConfigFile = OnceCell<Option<Ini>>;
type AwsCredsFile = OnceCell<Option<Ini>>;

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn get_credentials_file_path(ctx: &ChipContext) -> Option<PathBuf> {
    ctx
        .get_env("AWS_SHARED_CREDENTIALS_FILE")
        .or_else(|| ctx.get_env("AWS_CREDENTIALS_FILE"))
        .and_then(|path| PathBuf::from_str(&path).ok())
        .or_else(|| {
            let mut home = dirs::home_dir()?;
            home.push(".aws/credentials");
            Some(home)
        })
}

fn get_config_file_path(ctx: &ChipContext) -> Option<PathBuf> {
    ctx
        .get_env("AWS_CONFIG_FILE")
        .and_then(|path| PathBuf::from_str(&path).ok())
        .or_else(|| {
            let mut home = dirs::home_dir()?;
            home.push(".aws/config");
            Some(home)
        })
}

// Initialize the AWS config file once
fn get_config<'a>(ctx: &ChipContext, config: &'a OnceCell<Option<Ini>>) -> Option<&'a Ini> {
    config
        .get_or_init(|| {
            let path = get_config_file_path(ctx)?;
            Ini::load_from_file(path).ok()
        })
        .as_ref()
}

// Initialize the AWS credentials file once
fn get_creds<'a>(ctx: &ChipContext, config: &'a OnceCell<Option<Ini>>) -> Option<&'a Ini> {
    config
        .get_or_init(|| {
            let path = get_credentials_file_path(ctx)?;
            Ini::load_from_file(path).ok()
        })
        .as_ref()
}

// Get the section for a given profile name in the config file.
fn get_profile_config<'a>(
    config: &'a Ini,
    profile: Option<&Profile>,
) -> Option<&'a ini::Properties> {
    match profile {
        Some(profile) => config.section(Some(format!("profile {profile}"))),
        None => config.section(Some("default")),
    }
}

// Get the section for a given profile name in the credentials file.
fn get_profile_creds<'a>(
    config: &'a Ini,
    profile: Option<&Profile>,
) -> Option<&'a ini::Properties> {
    match profile {
        None => config.section(Some("default")),
        _ => config.section(profile),
    }
}

fn get_aws_region_from_config(
    ctx: &ChipContext,
    aws_profile: &Option<Profile>,
    aws_config: &AwsConfigFile,
) -> Option<Region> {
    let config = get_config(ctx, aws_config)?;
    let section = get_profile_config(config, aws_profile.as_ref())?;

    section.get("region").map(std::borrow::ToOwned::to_owned)
}

fn get_aws_profile_and_region(
    ctx: &ChipContext,
    aws_config: &AwsConfigFile,
) -> (Option<Profile>, Option<Region>) {
    let profile_env_vars = [
        "AWSU_PROFILE",
        "AWS_VAULT",
        "AWSUME_PROFILE",
        "AWS_PROFILE",
        "AWS_SSO_PROFILE",
    ];
    let region_env_vars = ["AWS_REGION", "AWS_DEFAULT_REGION"];
    let profile = profile_env_vars
        .iter()
        .find_map(|env_var| ctx.get_env(env_var));
    let region = region_env_vars
        .iter()
        .find_map(|env_var| ctx.get_env(env_var));
    match (profile, region) {
        (Some(p), Some(r)) => (Some(p), Some(r)),
        (None, Some(r)) => (None, Some(r)),
        (Some(p), None) => (
            Some(p.clone()),
            get_aws_region_from_config(ctx, &Some(p), aws_config),
        ),
        (None, None) => (None, get_aws_region_from_config(ctx, &None, aws_config)),
    }
}

// Get the SSO cache key input from a profile section, handling both direct sso_start_url
// and sso_session (which references a separate [sso-session <name>] section).
fn get_sso_cache_key_input(profile_section: &ini::Properties) -> Option<String> {
    profile_section
        .get("sso_session")
        .map(|s| s.to_string())
        .or_else(|| profile_section.get("sso_start_url").map(|s| s.to_string()))
}

fn get_credentials_duration(
    ctx: &ChipContext,
    aws_profile: Option<&Profile>,
    aws_config: &AwsConfigFile,
    aws_creds: &AwsCredsFile,
) -> Option<i64> {
    let expiration_env_vars = [
        "AWS_CREDENTIAL_EXPIRATION",
        "AWS_SESSION_EXPIRATION",
        "AWSUME_EXPIRATION",
    ];
    let expiration_date = if let Some(expiration_date) = expiration_env_vars
        .into_iter()
        .find_map(|env_var| ctx.get_env(env_var))
    {
        // get expiration from environment variables
        chrono::DateTime::parse_from_rfc3339(&expiration_date).ok()
    } else if let Some(section) =
        get_creds(ctx, aws_creds).and_then(|creds| get_profile_creds(creds, aws_profile))
    {
        // get expiration from credentials file
        let expiration_keys = ["expiration", "x_security_token_expires"];
        expiration_keys
            .iter()
            .find_map(|expiration_key| section.get(expiration_key))
            .and_then(|expiration| DateTime::parse_from_rfc3339(expiration).ok())
    } else {
        // get expiration from cached SSO credentials
        let config = get_config(ctx, aws_config)?;
        let section = get_profile_config(config, aws_profile)?;
        let cache_key_input = get_sso_cache_key_input(section)?;
        // https://github.com/boto/botocore/blob/d7ff05fac5bf597246f9e9e3fac8f22d35b02e64/botocore/utils.py#L3350
        let cache_key = hex_encode(&Sha1::digest(cache_key_input.as_bytes()));
        // https://github.com/aws/aws-cli/blob/b3421dcdd443db95999364e94266c0337b45cc43/awscli/customizations/sso/utils.py#L89
        let mut sso_cred_path = dirs::home_dir()?;
        sso_cred_path.push(format!(".aws/sso/cache/{cache_key}.json"));
        let sso_cred_json: json::Value =
            json::from_str(&std::fs::read_to_string(&sso_cred_path).ok()?).ok()?;
        let expires_at = sso_cred_json.get("expiresAt")?.as_str();
        DateTime::parse_from_rfc3339(expires_at?).ok()
    }?;

    Some(expiration_date.timestamp() - chrono::Local::now().timestamp())
}

fn alias_name(name: Option<String>, aliases: &HashMap<String, &str>) -> Option<String> {
    name.as_ref()
        .and_then(|n| aliases.get(n))
        .map(|&a| a.to_string())
        .or(name)
}

fn has_credential_process_or_sso(
    ctx: &ChipContext,
    aws_profile: Option<&Profile>,
    aws_config: &AwsConfigFile,
    aws_creds: &AwsCredsFile,
) -> Option<bool> {
    let config = get_config(ctx, aws_config)?;
    let credentials = get_creds(ctx, aws_creds);

    let empty_section = ini::Properties::new();
    // We use the aws_profile here because `get_profile_config()` treats None
    // as "special" and falls back to the "[default]"; otherwise this tries
    // to look up "[profile default]" which doesn't exist
    let config_section = get_profile_config(config, aws_profile).or(Some(&empty_section))?;

    let credential_section = match credentials {
        Some(credentials) => get_profile_creds(credentials, aws_profile),
        None => None,
    };

    Some(
        config_section.contains_key("credential_process")
            || config_section.contains_key("sso_session")
            || config_section.contains_key("sso_start_url")
            || credential_section?.contains_key("credential_process")
            || credential_section?.contains_key("sso_start_url"),
    )
}

fn has_defined_credentials(
    ctx: &ChipContext,
    aws_profile: Option<&Profile>,
    aws_creds: &AwsCredsFile,
) -> Option<bool> {
    let valid_env_vars = [
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
    ];

    // accept if set through environment variable
    if valid_env_vars
        .iter()
        .any(|env_var| ctx.get_env(env_var).is_some())
    {
        return Some(true);
    }

    let creds = get_creds(ctx, aws_creds)?;
    let section = get_profile_creds(creds, aws_profile)?;
    Some(section.contains_key("aws_access_key_id"))
}

// https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html#cli-configure-files-settings
fn has_source_profile(
    ctx: &ChipContext,
    aws_profile: Option<&Profile>,
    aws_config: &AwsConfigFile,
    aws_creds: &AwsCredsFile,
) -> Option<bool> {
    let config = get_config(ctx, aws_config)?;

    let config_section = get_profile_config(config, aws_profile)?;
    let source_profile = config_section
        .get("source_profile")
        .map(std::borrow::ToOwned::to_owned);

    let has_credential_process =
        has_credential_process_or_sso(ctx, source_profile.as_ref(), aws_config, aws_creds)
            .unwrap_or(false);
    let has_credentials =
        has_defined_credentials(ctx, source_profile.as_ref(), aws_creds).unwrap_or(false);

    Some(has_credential_process || has_credentials)
}

// ---------------------------------------------------------------------------
// ChipProvider implementation (adapted entry point)
// ---------------------------------------------------------------------------

pub struct AwsProvider;

impl ChipProvider for AwsProvider {
    fn id(&self) -> ChipId {
        "aws"
    }

    fn display_name(&self) -> &str {
        "AWS"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ["AWS_VAULT", "AWSU_PROFILE", "AWSUME_PROFILE", "AWS_SSO_PROFILE",
         "AWS_PROFILE", "AWS_REGION", "AWS_DEFAULT_REGION", "AWS_ACCESS_KEY_ID"]
            .iter()
            .any(|var| ctx.has_env(var))
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_aws_info(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Aws"),
            tooltip: Some("AWS profile and region".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_aws_info(ctx: &ChipContext) -> Option<String> {
    let aws_config: AwsConfigFile = OnceCell::new();
    let aws_creds: AwsCredsFile = OnceCell::new();

    let (aws_profile, aws_region) = get_aws_profile_and_region(ctx, &aws_config);
    if aws_profile.is_none() && aws_region.is_none() {
        return None;
    }

    // Build label: "profile (region)" or just "profile" or "(region)"
    match (aws_profile, aws_region) {
        (Some(profile), Some(region)) => Some(format!("{profile} ({region})")),
        (Some(profile), None) => Some(profile),
        (None, Some(region)) => Some(format!("({region})")),
        (None, None) => None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_name() {
        let mut aliases = HashMap::new();
        aliases.insert("us-east-1".to_string(), "use1");
        assert_eq!(alias_name(Some("us-east-1".into()), &aliases), Some("use1".to_string()));
        assert_eq!(alias_name(Some("eu-west-1".into()), &aliases), Some("eu-west-1".to_string()));
        assert_eq!(alias_name(None, &aliases), None);
    }
}

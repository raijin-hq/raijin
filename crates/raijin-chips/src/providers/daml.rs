use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

const DAML_SDK_VERSION: &str = "sdk-version";
const DAML_SDK_VERSION_ENV: &str = "DAML_SDK_VERSION";
const DAML_YAML: &str = "daml.yaml";

pub struct DamlProvider;

impl ChipProvider for DamlProvider {
    fn id(&self) -> ChipId {
        "daml"
    }

    fn display_name(&self) -> &str {
        "Daml"
    }

    fn detect_files(&self) -> &[&str] {
        &["daml.yaml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_daml_sdk_version(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Daml"),
            tooltip: Some("Daml SDK version".into()),
            ..ChipOutput::default()
        }
    }
}

fn get_daml_sdk_version(ctx: &ChipContext) -> Option<String> {
    ctx.get_env(DAML_SDK_VERSION_ENV)
        .or_else(|| read_sdk_version_from_daml_yaml(ctx))
}

fn read_sdk_version_from_daml_yaml(ctx: &ChipContext) -> Option<String> {
    let file_contents = std::fs::read_to_string(ctx.cwd.join(DAML_YAML)).ok()?;
    let daml_yaml = yaml_rust2::YamlLoader::load_from_str(&file_contents).ok()?;
    let sdk_version = daml_yaml.first()?[DAML_SDK_VERSION].as_str()?;
    Some(sdk_version.to_string())
}

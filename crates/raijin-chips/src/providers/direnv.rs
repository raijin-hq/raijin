use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct DirenvProvider;

impl ChipProvider for DirenvProvider {
    fn id(&self) -> ChipId {
        "direnv"
    }

    fn display_name(&self) -> &str {
        "Direnv"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("DIRENV_DIR")
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = ctx
            .get_env("DIRENV_DIR")
            .map(|dir| {
                let stripped = dir.strip_prefix('-').unwrap_or(&dir);
                PathBuf::from(stripped)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| stripped.to_string())
            })
            .unwrap_or_default();

        let state = get_direnv_state(ctx);
        let tooltip = state.map(|s| {
            let loaded = if s.loaded { "loaded" } else { "not loaded" };
            let allowed = match s.allowed {
                AllowStatus::Allowed => "allowed",
                AllowStatus::NotAllowed => "not allowed",
                AllowStatus::Denied => "denied",
            };
            format!("direnv {loaded}/{allowed}")
        });

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Settings"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

fn get_direnv_state(ctx: &ChipContext) -> Option<DirenvState> {
    let direnv_status = &ctx.exec_cmd("direnv", &["status", "--json"])?.stdout;
    serde_json::from_str::<RawDirenvState>(direnv_status)
        .map_or_else(
            |_| {
                DirenvState::from_lines(direnv_status)
                    .ok()
            },
            |raw| {
                raw.into_direnv_state()
                    .ok()
                    .flatten()
            },
        )
}

struct DirenvState {
    pub rc_path: PathBuf,
    pub allowed: AllowStatus,
    pub loaded: bool,
}

impl FromStr for DirenvState {
    type Err = Cow<'static, str>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match serde_json::from_str::<RawDirenvState>(s) {
            Ok(raw) => raw
                .into_direnv_state()?
                .ok_or_else(|| Cow::from("unknown direnv state")),
            Err(_) => Self::from_lines(s),
        }
    }
}

impl DirenvState {
    fn from_lines(s: &str) -> Result<Self, Cow<'static, str>> {
        let mut rc_path = PathBuf::new();
        let mut allowed = None;
        let mut loaded = true;

        for line in s.lines() {
            if let Some(path) = line.strip_prefix("Found RC path") {
                rc_path = PathBuf::from_str(path.trim()).map_err(|e| Cow::from(e.to_string()))?
            } else if let Some(value) = line.strip_prefix("Found RC allowed") {
                allowed = Some(AllowStatus::from_str(value.trim())?);
            } else if line.contains("No .envrc or .env loaded") {
                loaded = false;
            };
        }

        if rc_path.as_os_str().is_empty() || allowed.is_none() {
            return Err(Cow::from("unknown direnv state"));
        }

        Ok(Self {
            rc_path,
            allowed: allowed.unwrap(),
            loaded,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AllowStatus {
    Allowed,
    NotAllowed,
    Denied,
}

impl FromStr for AllowStatus {
    type Err = Cow<'static, str>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" | "true" => Ok(Self::Allowed),
            "1" => Ok(Self::NotAllowed),
            "2" | "false" => Ok(Self::Denied),
            _ => Err(Cow::from("invalid allow status")),
        }
    }
}

impl TryFrom<u8> for AllowStatus {
    type Error = Cow<'static, str>;

    fn try_from(u: u8) -> Result<Self, Self::Error> {
        match u {
            0 => Ok(Self::Allowed),
            1 => Ok(Self::NotAllowed),
            2 => Ok(Self::Denied),
            _ => Err(Cow::from("unknown integer allow status")),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawDirenvState {
    pub state: State,
}

impl RawDirenvState {
    fn into_direnv_state(self) -> Result<Option<DirenvState>, Cow<'static, str>> {
        match (self.state.found_rc, self.state.loaded_rc) {
            (None, None) => Ok(None),
            (Some(found_rc), None) => Ok(Some(DirenvState {
                rc_path: found_rc.path,
                allowed: found_rc.allowed.try_into()?,
                loaded: false,
            })),
            (Some(found_rc), Some(loaded_rc)) => Ok(Some(DirenvState {
                rc_path: found_rc.path,
                allowed: found_rc.allowed.try_into()?,
                loaded: matches!(loaded_rc.allowed.try_into()?, AllowStatus::Allowed),
            })),
            (None, Some(_)) => Err(Cow::from("unknown direnv state")),
        }
    }
}

#[derive(Debug, Deserialize)]
struct State {
    #[serde(rename = "foundRC")]
    pub found_rc: Option<RCStatus>,
    #[serde(rename = "loadedRC")]
    pub loaded_rc: Option<RCStatus>,
}

#[derive(Debug, Deserialize)]
struct RCStatus {
    pub allowed: u8,
    pub path: PathBuf,
}

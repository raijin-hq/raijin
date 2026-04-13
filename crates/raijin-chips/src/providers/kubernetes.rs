
use serde_json::Value as JsonValue;
use yaml_rust2::{Yaml, YamlLoader};

use std::borrow::Cow;
use std::env;
use std::path::PathBuf;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

#[derive(Default)]
struct KubeCtxComponents {
    user: Option<String>,
    namespace: Option<String>,
    cluster: Option<String>,
}

fn get_current_kube_context_name<T: DataValue>(document: &T) -> Option<&str> {
    document
        .get("current-context")
        .and_then(DataValue::as_str)
        .filter(|s| !s.is_empty())
}

fn get_kube_ctx_components<T: DataValue>(
    document: &T,
    current_ctx_name: &str,
) -> Option<KubeCtxComponents> {
    document
        .get("contexts")?
        .as_array()?
        .iter()
        .find(|ctx| ctx.get("name").and_then(DataValue::as_str) == Some(current_ctx_name))
        .map(|ctx| KubeCtxComponents {
            user: ctx
                .get("context")
                .and_then(|v| v.get("user"))
                .and_then(DataValue::as_str)
                .map(String::from),
            namespace: ctx
                .get("context")
                .and_then(|v| v.get("namespace"))
                .and_then(DataValue::as_str)
                .map(String::from),
            cluster: ctx
                .get("context")
                .and_then(|v| v.get("cluster"))
                .and_then(DataValue::as_str)
                .map(String::from),
        })
}

fn get_aliased_name<'a>(
    pattern: Option<&'a str>,
    current_value: Option<&str>,
    alias: Option<&'a str>,
) -> Option<String> {
    let replacement = alias.or(current_value)?.to_string();
    let Some(pattern) = pattern else {
        // If user pattern not set, treat it as a match-all pattern
        return Some(replacement);
    };
    // If a pattern is set, but we have no value, there is no match
    let value = current_value?;
    if value == pattern {
        return Some(replacement);
    }
    let re = match regex::Regex::new(&format!("^{pattern}$")) {
        Ok(re) => re,
        Err(error) => {
            log::warn!(
                "Could not compile regular expression `{}`:\n{}",
                &format!("^{pattern}$"),
                error
            );
            return None;
        }
    };
    let replaced = re.replace(value, replacement.as_str());
    match replaced {
        Cow::Owned(replaced) => Some(replaced),
        // It didn't match...
        Cow::Borrowed(_) => None,
    }
}

#[derive(Debug)]
enum Document {
    Json(JsonValue),
    Yaml(Yaml),
}

trait DataValue {
    fn get(&self, key: &str) -> Option<&Self>;
    fn as_str(&self) -> Option<&str>;
    fn as_array(&self) -> Option<Vec<&Self>>;
}

impl DataValue for JsonValue {
    fn get(&self, key: &str) -> Option<&Self> {
        self.get(key)
    }

    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }

    fn as_array(&self) -> Option<Vec<&Self>> {
        self.as_array().map(|arr| arr.iter().collect())
    }
}

impl DataValue for Yaml {
    fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Hash(map) => map.get(&Self::String(key.to_string())),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }

    fn as_array(&self) -> Option<Vec<&Self>> {
        match self {
            Self::Array(arr) => Some(arr.iter().collect()),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// ChipProvider implementation (adapted entry point)
// ---------------------------------------------------------------------------

pub struct KubernetesProvider;

impl ChipProvider for KubernetesProvider {
    fn id(&self) -> ChipId {
        "kubernetes"
    }

    fn display_name(&self) -> &str {
        "Kubernetes"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("KUBECONFIG")
            || dirs::home_dir()
                .map(|h| h.join(".kube").join("config").exists())
                .unwrap_or(false)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_kube_info(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Kubernetes"),
            tooltip: Some("Kubernetes context".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_kube_info(ctx: &ChipContext) -> Option<String> {
    let default_config_file = dirs::home_dir()?.join(".kube").join("config");

    let kube_cfg = ctx
        .get_env("KUBECONFIG")
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_config_file.to_string_lossy().to_string());

    let raw_kubeconfigs = env::split_paths(&kube_cfg)
        .map(|file| std::fs::read_to_string(file).ok());
    let kubeconfigs = parse_kubeconfigs(raw_kubeconfigs);

    let current_kube_ctx_name = kubeconfigs.iter().find_map(|v| match v {
        Document::Json(json) => get_current_kube_context_name(json),
        Document::Yaml(yaml) => get_current_kube_context_name(yaml),
    })?;

    // Even if we have multiple config files, the first key wins
    // https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/
    let ctx_components: KubeCtxComponents = kubeconfigs
        .iter()
        .find_map(|kubeconfig| match kubeconfig {
            Document::Json(json) => get_kube_ctx_components(json, current_kube_ctx_name),
            Document::Yaml(yaml) => get_kube_ctx_components(yaml, current_kube_ctx_name),
        })
        .unwrap_or_else(|| {
            log::warn!(
                "Invalid KUBECONFIG: identified current-context `{}`, but couldn't find the context in config file(s): `{}`",
                current_kube_ctx_name,
                &kube_cfg,
            );
            KubeCtxComponents::default()
        });

    // Build label: "context (namespace)"
    let display_context = current_kube_ctx_name.to_string();
    match &ctx_components.namespace {
        Some(ns) if !ns.is_empty() => Some(format!("{display_context} ({ns})")),
        _ => Some(display_context),
    }
}

fn parse_kubeconfigs<I>(raw_kubeconfigs: I) -> Vec<Document>
where
    I: Iterator<Item = Option<String>>,
{
    raw_kubeconfigs
        .filter_map(|content| match content {
            Some(value) => match value.chars().next() {
                // Parsing as json is about an order of magnitude faster than parsing
                // as yaml, so do that if possible.
                Some('{') => match serde_json::from_str(&value) {
                    Ok(json) => Some(Document::Json(json)),
                    Err(_) => parse_yaml(&value),
                },
                _ => parse_yaml(&value),
            },
            _ => None,
        })
        .collect()
}

fn parse_yaml(s: &str) -> Option<Document> {
    YamlLoader::load_from_str(s)
        .ok()
        .and_then(|yaml| yaml.into_iter().next().map(Document::Yaml))
}

mod deprecated {
    use std::borrow::Cow;
    use std::collections::HashMap;

    pub fn get_alias<'a>(
        current_value: String,
        aliases: &'a HashMap<String, &'a str>,
        name: &'a str,
    ) -> Option<String> {
        let alias = if let Some(val) = aliases.get(current_value.as_str()) {
            // simple match without regex
            Some((*val).to_string())
        } else {
            // regex match
            aliases.iter().find_map(|(k, v)| {
                let re = regex::Regex::new(&format!("^{k}$")).ok()?;
                let replaced = re.replace(current_value.as_str(), *v);
                match replaced {
                    // We have a match if the replaced string is different from the original
                    Cow::Owned(replaced) => Some(replaced),
                    Cow::Borrowed(_) => None,
                }
            })
        };

        match alias {
            Some(alias) => {
                log::warn!(
                    "Usage of '{}_aliases' is deprecated and will be removed in 2.0; Use 'contexts' with '{}_alias' instead. (`{}` -> `{}`)",
                    &name,
                    &name,
                    &current_value,
                    &alias
                );
                Some(alias)
            }
            None => Some(current_value),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml_kubeconfig() {
        let yaml = r#"
apiVersion: v1
current-context: test-context
contexts:
- context:
    cluster: test-cluster
    namespace: test-ns
    user: test-user
  name: test-context
"#;
        let docs = parse_kubeconfigs(std::iter::once(Some(yaml.to_string())));
        assert_eq!(docs.len(), 1);

        let ctx_name = match &docs[0] {
            Document::Yaml(y) => get_current_kube_context_name(y),
            _ => None,
        };
        assert_eq!(ctx_name, Some("test-context"));

        let components = match &docs[0] {
            Document::Yaml(y) => get_kube_ctx_components(y, "test-context"),
            _ => None,
        };
        let c = components.unwrap();
        assert_eq!(c.namespace.as_deref(), Some("test-ns"));
        assert_eq!(c.user.as_deref(), Some("test-user"));
        assert_eq!(c.cluster.as_deref(), Some("test-cluster"));
    }

    #[test]
    fn parse_json_kubeconfig() {
        let json = r#"{
            "apiVersion": "v1",
            "current-context": "json-ctx",
            "contexts": [{
                "name": "json-ctx",
                "context": {
                    "cluster": "json-cluster",
                    "namespace": "json-ns",
                    "user": "json-user"
                }
            }]
        }"#;
        let docs = parse_kubeconfigs(std::iter::once(Some(json.to_string())));
        assert_eq!(docs.len(), 1);

        let ctx_name = match &docs[0] {
            Document::Json(j) => get_current_kube_context_name(j),
            _ => None,
        };
        assert_eq!(ctx_name, Some("json-ctx"));

        let components = match &docs[0] {
            Document::Json(j) => get_kube_ctx_components(j, "json-ctx"),
            _ => None,
        };
        let c = components.unwrap();
        assert_eq!(c.namespace.as_deref(), Some("json-ns"));
        assert_eq!(c.user.as_deref(), Some("json-user"));
        assert_eq!(c.cluster.as_deref(), Some("json-cluster"));
    }

    #[test]
    fn alias_exact_match() {
        let result = get_aliased_name(Some("production"), Some("production"), Some("prod"));
        assert_eq!(result, Some("prod".to_string()));
    }

    #[test]
    fn alias_regex_match() {
        let result = get_aliased_name(
            Some("arn:aws:eks:.*:.*:cluster/(.*)"),
            Some("arn:aws:eks:us-east-1:123456789:cluster/my-cluster"),
            Some("$1"),
        );
        assert_eq!(result, Some("my-cluster".to_string()));
    }

    #[test]
    fn alias_no_match() {
        let result = get_aliased_name(Some("staging"), Some("production"), Some("stg"));
        assert!(result.is_none());
    }

    #[test]
    fn multi_config_first_context_wins() {
        let yaml1 = "current-context: from-first\ncontexts:\n- name: from-first\n  context:\n    namespace: ns1\n";
        let yaml2 = "current-context: from-second\ncontexts:\n- name: from-second\n  context:\n    namespace: ns2\n";

        let docs = parse_kubeconfigs(
            vec![Some(yaml1.to_string()), Some(yaml2.to_string())].into_iter(),
        );

        let ctx_name = docs.iter().find_map(|v| match v {
            Document::Yaml(y) => get_current_kube_context_name(y),
            _ => None,
        });
        assert_eq!(ctx_name, Some("from-first"));
    }

    #[test]
    fn empty_context_name_ignored() {
        let yaml = "current-context: \"\"\ncontexts: []\n";
        let docs = parse_kubeconfigs(std::iter::once(Some(yaml.to_string())));
        let ctx_name = docs.iter().find_map(|v| match v {
            Document::Yaml(y) => get_current_kube_context_name(y),
            _ => None,
        });
        assert_eq!(ctx_name, None);
    }
}

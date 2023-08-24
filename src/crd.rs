use std::collections::BTreeMap;

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub static FORWARDED_SERVICE_FINALIZER: &str = "forwardedservices.port-forward-operator.rs";
#[allow(dead_code)]
pub const ANNOTATION_GENERATION: &str = "port-forward-operator.rs/observed-generation";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[cfg_attr(test, derive(Default))]
#[kube(
    kind = "ForwardedService",
    group = "port-forward-operator.rs",
    version = "v1",
    namespaced
)]
#[kube(status = "ForwardedServiceStatus", shortname = "fwd")]
pub struct ForwardedServiceSpec {
    pub service: String,
    pub namespace: Option<String>,
    #[schemars(length(min = 1), schema_with = "ports")]
    pub ports: Vec<String>,
    pub kube_config: KubeConfigReference,
}

fn ports(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    serde_json::from_value(serde_json::json!({
        "type": "array",
        "items": {
            "type": "string",
            "pattern": "\\d{0,5}(:(\\d{0,5}))?"
        }
    }))
    .unwrap()
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct KubeConfigReference {
    pub secret: String,
    pub key: Option<String>,
    pub context: String,
    pub user: Option<String>,
    pub cluster: Option<String>,
}

/// The status object of `ForwardedService`
#[derive(Deserialize, Serialize, Clone, Default, Debug, JsonSchema)]
pub struct ForwardedServiceStatus {
    pub service_name: String,
    pub pod_name: String,
}

impl KubeConfigReference {
    #[allow(dead_code)]
    pub(crate) fn key_any(&self) -> String {
        self.key.clone().unwrap_or("config".to_owned())
    }
}

impl ForwardedService {
    #[allow(dead_code)]
    pub(crate) fn annotate(&self) -> BTreeMap<String, String> {
        let mut annotations: BTreeMap<String, String> = BTreeMap::new();
        annotations.insert(
            ANNOTATION_GENERATION.to_owned(),
            self.metadata.generation.unwrap_or_default().to_string(),
        );
        annotations
    }
}

impl Default for KubeConfigReference {
    fn default() -> Self {
        Self {
            secret: Default::default(),
            key: Some("config".to_owned()),
            context: String::new(),
            user: None,
            cluster: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KubeConfigReference;

    #[test]
    fn test_key_any_on_set_key() {
        let reference = KubeConfigReference {
            secret: "secret".to_owned(),
            key: Some("key".to_owned()),
            context: "context".to_owned(),
            user: None,
            cluster: None,
        };
        let value = reference.key_any();
        let key = reference.key.unwrap();
        assert_eq!(key, value);
    }

    #[test]
    fn test_key_any_on_no_key() {
        let reference = KubeConfigReference {
            secret: "secret".to_owned(),
            key: None,
            context: "context".to_owned(),
            user: None,
            cluster: None,
        };
        let value = reference.key_any();
        assert_eq!("config", value);
    }
}

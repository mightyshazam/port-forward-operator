use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use futures::StreamExt;
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, DeploymentSpec},
        core::v1::{
            Container, PodSpec, SecretVolumeSource, Service, ServicePort, Volume, VolumeMount,
        },
    },
    apimachinery::pkg::apis::meta::v1::{LabelSelector, OwnerReference},
};
use kube::{
    api::{ListParams, Patch, PatchParams, PostParams},
    core::{CustomResourceExt, ObjectMeta},
    runtime::finalizer::Event as Finalizer,
    runtime::{
        controller::Action,
        events::{Event, EventType, Recorder, Reporter},
        finalizer,
        watcher::Config,
        Controller,
    },
    Api, Client, Resource, ResourceExt,
};
use tokio::sync::RwLock;

pub mod host;
mod state;
use self::state::State;
use crate::{
    crd::{ForwardedService, ANNOTATION_GENERATION, FORWARDED_SERVICE_FINALIZER},
    error::Error,
};
use serde::{de::DeserializeOwned, Serialize};

const KUBE_CONFIG_PATH: &str = "/etc/port-forward-operator/kube";

pub fn new_state(image: String) -> State {
    State::new(image)
}

#[derive(Clone, Serialize)]
pub struct Diagnostics {
    #[serde(deserialize_with = "from_ts")]
    pub last_event: DateTime<Utc>,
    #[serde(skip)]
    pub reporter: Reporter,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            last_event: Utc::now(),
            reporter: "forwardedservice-controller".into(),
        }
    }
}
impl Diagnostics {
    fn recorder(&self, client: Client, doc: &ForwardedService) -> Recorder {
        Recorder::new(client, self.reporter.clone(), doc.object_ref(&()))
    }
}

#[derive(Clone)]
pub struct Context {
    /// Kubernetes client
    pub client: Client,
    /// Diagnostics read by the web server
    pub diagnostics: Arc<RwLock<Diagnostics>>,

    /// Image
    pub image: String,
    // Prometheus metrics
    // pub metrics: Metrics,
}

pub async fn start(controller_state: State) {
    let client = Client::try_default()
        .await
        .map_err(|e| Error::KubeClient { source: e })
        .expect("failed to create kubernetes client");
    let api = Api::<ForwardedService>::all(client.clone());
    if let Err(e) = api.list(&ListParams::default().limit(1)).await {
        tracing::error!("Installation: cargo run --bin crdgen | kubectl apply -f -");
        panic!("crds are not installed: {}", Error::KubeCrd { source: e });
    }

    Controller::new(api, Config::default().any_semantic())
        .shutdown_on_signal()
        .run(reconcile, error_policy, controller_state.to_context(client))
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .await;
}

fn error_policy(_: Arc<ForwardedService>, error: &Error, _: Arc<Context>) -> Action {
    tracing::warn!("reconcile failed: {:?}", error);
    // ctx.metrics.reconcile_failure(&doc, error);
    Action::requeue(Duration::from_secs(5 * 60))
}

async fn reconcile(svc: Arc<ForwardedService>, ctx: Arc<Context>) -> Result<Action, Error> {
    ctx.diagnostics.write().await.last_event = Utc::now();
    let ns = svc.namespace().unwrap(); // doc is namespace scoped
    let docs: Api<ForwardedService> = Api::namespaced(ctx.client.clone(), &ns);

    tracing::info!(
        "Reconciling ForwardedService \"{}\" in {}",
        svc.name_any(),
        ns
    );
    finalizer(&docs, FORWARDED_SERVICE_FINALIZER, svc, |event| async {
        match event {
            Finalizer::Apply(doc) => doc.reconcile(ctx.clone()).await,
            Finalizer::Cleanup(doc) => doc.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}

impl ForwardedService {
    async fn create_or_update<T>(
        &self,
        api: &Api<T>,
        obj: T,
        convert: impl FnOnce(&ForwardedService, &T, &T) -> bool,
    ) -> Result<T, Error>
    where
        T: Resource + Serialize + DeserializeOwned + Clone + std::fmt::Debug,
    {
        let name = &obj.name_any();
        match api.create(&PostParams::default(), &obj).await {
            Ok(res) => Ok(res),
            Err(e) => match e {
                kube::Error::Api(r) if r.code == 409 => {
                    let data = api
                        .get(name)
                        .await
                        .map_err(|k| Error::Kubernetes { source: k })?;

                    if convert(self, &data, &obj) {
                        tracing::info!(
                            "updating object {}/{} due to differences",
                            data.namespace().unwrap(),
                            data.name_any()
                        );
                        api.patch(
                            name,
                            &PatchParams::apply("port-forward-operator"),
                            &Patch::Merge(obj),
                        )
                        .await
                        .map_err(|k| Error::Kubernetes { source: k })
                    } else {
                        Ok(obj)
                    }
                }
                _ => Err(Error::Kubernetes { source: e }),
            },
        }
    }

    fn compare_generation<T: Resource>(fs: &Self, actual: &T, _: &T) -> bool {
        let generation = fs.metadata.generation.unwrap_or_default().to_string();
        match actual.annotations().get(ANNOTATION_GENERATION) {
            Some(gen) => gen != &generation,
            None => true,
        }
    }

    async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action, Error> {
        let client = ctx.client.clone();
        let recorder = ctx.diagnostics.read().await.recorder(client.clone(), self);
        let ns = self.namespace().unwrap();
        let name = self.name_any();
        let deployments: Api<Deployment> = Api::namespaced(client.clone(), &ns);
        let services: Api<Service> = Api::namespaced(client.clone(), &ns);
        let (service, pod) = self.create_service_and_deployment(ctx.as_ref())?;
        let _ = self
            .create_or_update(&deployments, pod, |fs, actual, expected| {
                if Self::compare_generation(fs, actual, expected) {
                    true
                } else {
                    let actual_spec = actual
                        .spec
                        .as_ref()
                        .unwrap()
                        .template
                        .spec
                        .as_ref()
                        .unwrap();
                    let expected_spec = expected
                        .spec
                        .as_ref()
                        .unwrap()
                        .template
                        .spec
                        .as_ref()
                        .unwrap();
                    match actual_spec.containers[0].image != expected_spec.containers[0].image
                        || actual_spec.containers[0].args == expected_spec.containers[0].args
                    {
                        true => true,
                        false => false,
                    }
                }
            })
            .await?;
        let _ = self
            .create_or_update(&services, service, Self::compare_generation)
            .await?;

        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "ForwardingRequest".into(),
                note: Some(format!("Forwarding `{name}`")),
                action: "Forwarding".into(),
                secondary: None,
            })
            .await
            .map_err(|e| Error::Kubernetes { source: e })?;

        Ok(Action::requeue(Duration::from_secs(300)))
    }

    fn add_vector_args(&self, args: &mut Vec<String>) {
        let ns = self.spec.namespace.clone();
        args.push("--kubeconfig".to_owned());
        args.push(format!(
            "{}/{}",
            KUBE_CONFIG_PATH,
            self.spec.kube_config.key_any()
        ));
        args.push("--context".to_owned());
        args.push(self.spec.kube_config.context.clone());
        if let Some(kube_user) = &self.spec.kube_config.user {
            args.push("--user".to_owned());
            args.push(kube_user.clone());
        }

        if let Some(kube_cluster) = &self.spec.kube_config.cluster {
            args.push("--cluster".to_owned());
            args.push(kube_cluster.clone());
        }

        args.push("--namespace".to_owned());
        args.push(ns.or(self.namespace()).unwrap());
        args.push("port-forward".to_owned());
        args.push("--address".into());
        args.push("0.0.0.0".into());
        args.push(format!("svc/{}", self.spec.service.clone()));
    }

    fn create_service_and_deployment(&self, ctx: &Context) -> Result<(Service, Deployment), Error> {
        let mut labels: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut ports: Vec<ServicePort> = Vec::new();
        let mut args: Vec<String> = Vec::with_capacity(self.spec.ports.len() + 6);
        self.add_vector_args(&mut args);
        for port in &self.spec.ports {
            args.push(port.to_string());
            let from = match port.split_once(':') {
                Some((first, _second)) => first,
                None => port,
            };

            let int_port: i32 = from.parse().unwrap();
            ports.push(ServicePort {
                name: Some(port.to_string().replace(':', "-")),
                port: int_port,
                protocol: Some("TCP".to_owned()),
                target_port: Some(
                    k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(int_port),
                ),
                app_protocol: None,
                ..Default::default()
            });
        }

        labels.insert(
            "port-forward-operator.rs/forwardedservice".to_owned(),
            self.name_any(),
        );
        let api_resource = Self::api_resource();
        let new_service = Service {
            metadata: kube::core::ObjectMeta {
                annotations: Some(self.annotate()),
                finalizers: None,
                labels: Some(labels.clone()),
                name: Some(self.name_any()),
                namespace: self.namespace(),
                owner_references: Some(vec![OwnerReference {
                    api_version: api_resource.api_version.clone(),
                    controller: Some(false),
                    kind: api_resource.kind.clone(),
                    name: self.name_any(),
                    uid: self.meta().uid.clone().unwrap(),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ServiceSpec {
                ports: Some(ports),
                selector: Some(labels.clone()),
                session_affinity: None,
                type_: Some("ClusterIP".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let new_deployment = Deployment {
            metadata: kube::core::ObjectMeta {
                annotations: Some(self.annotate()),
                finalizers: None,
                labels: Some(labels.clone()),
                name: Some(self.name_any()),
                namespace: self.namespace(),
                owner_references: Some(vec![OwnerReference {
                    api_version: api_resource.api_version.clone(),
                    controller: Some(false),
                    kind: api_resource.kind,
                    name: self.name_any(),
                    uid: self.meta().uid.clone().unwrap(),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            spec: Some(DeploymentSpec {
                revision_history_limit: Some(2),
                selector: LabelSelector {
                    match_expressions: None,
                    match_labels: Some(labels.clone()),
                },
                template: k8s_openapi::api::core::v1::PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            args: Some(args),
                            command: None,
                            env: None,
                            image: Some(ctx.image.clone()),
                            name: "forwarder".to_owned(),
                            ports: None,
                            volume_mounts: Some(vec![VolumeMount {
                                mount_path: KUBE_CONFIG_PATH.to_owned(),
                                name: "kubeconfig".to_owned(),
                                read_only: Some(true),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        }],
                        restart_policy: None,
                        volumes: Some(vec![Volume {
                            name: "kubeconfig".to_owned(),
                            secret: Some(SecretVolumeSource {
                                optional: Some(false),
                                secret_name: Some(self.spec.kube_config.secret.clone()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        };
        Ok((new_service, new_deployment))
    }

    async fn cleanup(&self, ctx: Arc<Context>) -> Result<Action, Error> {
        let recorder = ctx
            .diagnostics
            .read()
            .await
            .recorder(ctx.client.clone(), self);
        // Document doesn't have any real cleanup, so we just publish an event
        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "DeleteRequested".into(),
                note: Some(format!("Delete `{}`", self.name_any())),
                action: "Deleting".into(),
                secondary: None,
            })
            .await
            .map_err(|e| Error::Kubernetes { source: e })?;
        Ok(Action::await_change())
    }
}

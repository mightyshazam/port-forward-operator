use k8s_openapi::api::{
    core::v1::{Endpoints, Pod, Service},
    discovery::v1::{Endpoint, EndpointSlice},
};
use kube::{api::ListParams, config::KubeConfigOptions, Api, Client, Config};

use crate::error::Error;

pub(crate) struct ServiceOptions {
    namespace: String,
    name: String,
    ports: Vec<u16>,
    max_retries: Option<i32>,
}

impl ServiceOptions {
    pub fn new(namespace: String, name: String, ports: Vec<u16>, max_retries: Option<i32>) -> Self {
        Self {
            namespace,
            name,
            ports,
            max_retries,
        }
    }
}

pub(crate) async fn start(
    service_options: ServiceOptions,
    kube_options: &KubeConfigOptions,
) -> Result<(), Error> {
    let max_retries = service_options.max_retries.unwrap_or(3);
    let cfg = Config::from_kubeconfig(kube_options).await?;

    let client = Client::try_from(cfg).unwrap();
    let api = Api::<Pod>::namespaced(client, &service_options.namespace);
    let mut attempts = 1;
    loop {
        if attempts > max_retries {
            return Err(Error::MaxAttempts(max_retries));
        }

        match api
            .portforward(&service_options.name, &service_options.ports)
            .await
        {
            Ok(pf) => {
                attempts = 1;
                if let Err(e) = pf.join().await {
                    tracing::warn!(
                        "join error {}/{}: {}",
                        &service_options.namespace,
                        &service_options.name,
                        e
                    );
                } else {
                    tracing::info!("port forwarding complete");
                    return Ok(());
                }
            }
            Err(e) => {
                attempts += 1;
                tracing::warn!(
                    "unable to start port forward to service {}/{}: {}",
                    &service_options.namespace,
                    &service_options.name,
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await
            }
        }
    }
}

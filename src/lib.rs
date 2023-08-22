mod controller;
mod crd;
mod error;
mod service;

type Result<T> = std::result::Result<T, error::Error>;

pub async fn start_controller(image: String, listen_address: String) -> Result<()> {
    let b = Box::new(listen_address);
    let jh = tokio::spawn(controller::host::start_host(Box::leak(b)));
    controller::start(controller::new_state(image)).await;
    jh.await.unwrap()
}

pub async fn start_service(
    namespace: String,
    name: String,
    ports: Vec<u16>,
    max_retries: Option<i32>,
    kube_config: kube::config::KubeConfigOptions,
) -> Result<()> {
    tracing_subscriber::fmt::init();
    service::start(
        service::ServiceOptions::new(namespace, name, ports, max_retries),
        &kube_config,
    )
    .await
}

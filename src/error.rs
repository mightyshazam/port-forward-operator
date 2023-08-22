use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid kubernetes configuration: {source}")]
    KubeConfig {
        #[from]
        source: kube::config::KubeconfigError,
    },
    #[error("unable to create kubernetes client: {source}")]
    KubeClient { source: kube::Error },
    #[error("unable to query kubernetes crd: {source}")]
    KubeCrd { source: kube::Error },
    #[error("kubernetes error: {source}")]
    Kubernetes { source: kube::Error },
    #[error("failed to establish port forward after {0} attempts")]
    MaxAttempts(i32),
    #[error("server error: {0}")]
    Server(String),
    #[error("finalizer error: {0}")]
    FinalizerError(#[source] Box<kube::runtime::finalizer::Error<Error>>),
    #[error("service `{name}` error: {message}")]
    InvalidService { name: String, message: String },
}

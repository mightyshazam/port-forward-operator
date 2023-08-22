use clap::Parser;
use port_forward_operator::{start_controller, start_service};
mod app;

#[tokio::main]
async fn main() {
    let args = app::Arguments::parse();
    let result = match args.cmd {
        app::SubCommand::Controller {
            listen_address,
            image,
        } => start_controller(image, listen_address).await,
        app::SubCommand::Service {
            namespace,
            name,
            ports,
            max_retries,
            kube_context,
            kube_user,
            kube_cluster,
        } => {
            start_service(
                namespace,
                name,
                ports,
                Some(max_retries),
                kube::config::KubeConfigOptions {
                    context: Some(kube_context),
                    cluster: kube_cluster,
                    user: kube_user,
                },
            )
            .await
        }
    };
}

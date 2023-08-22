use clap::{Parser, Subcommand};
#[derive(Subcommand, Debug)]
pub enum SubCommand {
    Controller {
        #[clap(long, env, required = false, default_value = "0.0.0.0:8080")]
        listen_address: String,
        #[clap(long, env, required = true)]
        image: String,
    },
    Service {
        #[clap(long, env, required = true)]
        namespace: String,
        #[clap(long, env, required = true)]
        name: String,
        #[clap(long, env, required = true)]
        ports: Vec<u16>,
        #[clap(long, env, default_value_t = 3)]
        max_retries: i32,
        #[clap(long, env, required = true)]
        kube_context: String,
        #[clap(long, env)]
        kube_user: Option<String>,
        #[clap(long, env)]
        kube_cluster: Option<String>,
    },
}

#[derive(Parser, Debug)]
#[clap(author = "Author Name", version, about)]
/// A Very simple Package Hunter
pub struct Arguments {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

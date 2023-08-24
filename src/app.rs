use clap::{Parser, Subcommand};
const DEFAULT_LISTEN_ADDRES: &str = "0.0.0.0:8080";

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    Controller {
        #[clap(long, env, required = false, default_value = DEFAULT_LISTEN_ADDRES)]
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
/// A controller for port forwarding
pub struct Arguments {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[cfg(test)]
mod tests {
    use crate::app::DEFAULT_LISTEN_ADDRES;

    use super::Arguments;
    use clap::Parser;

    #[derive(Debug, Clone)]
    enum Error {
        IncorrectCommand,
    }

    #[test]
    fn test_controller_all_arguments() {
        get_subcommand_matches(
            make_args(&mut vec![
                "controller",
                "--listen-address",
                "0.0.0.0:80",
                "--image",
                "test",
            ]),
            |listen_address, image| {
                assert_eq!("0.0.0.0:80", listen_address);
                assert_eq!("test", image);
            },
        )
        .expect("there should be no errors");
    }

    #[test]
    fn test_controller_image_argument() {
        get_subcommand_matches(
            make_args(&mut vec!["controller", "--image", "test"]),
            |listen_address, image| {
                assert_eq!(DEFAULT_LISTEN_ADDRES, listen_address);
                assert_eq!("test", image);
            },
        )
        .expect("there should be no errors");
    }

    #[test]
    fn test_controller_image_from_environment() {
        let image_name = "something-interesting";
        std::env::set_var("IMAGE", image_name);
        get_subcommand_matches(
            make_args(&mut vec!["controller"]),
            |listen_address, image| {
                assert_eq!(DEFAULT_LISTEN_ADDRES, listen_address);
                assert_eq!(image_name, image);
            },
        )
        .expect("there should be no errors");
    }

    fn get_subcommand_matches(
        args: Vec<&str>,
        assertions: impl FnOnce(String, String),
    ) -> Result<(), Error> {
        let arguments = Arguments::parse_from(args);
        match arguments.cmd {
            super::SubCommand::Controller {
                listen_address,
                image,
            } => {
                assertions(listen_address, image);
                Ok(())
            }
            super::SubCommand::Service {
                namespace: _,
                name: _,
                ports: _,
                max_retries: _,
                kube_context: _,
                kube_user: _,
                kube_cluster: _,
            } => Err(Error::IncorrectCommand),
        }
    }

    fn make_args(input: &mut Vec<&'static str>) -> Vec<&'static str> {
        let mut args: Vec<&'static str> = vec!["app"];
        args.append(input);
        args
    }
}

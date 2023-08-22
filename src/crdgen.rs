use kube::CustomResourceExt;
mod crd;
fn main() {
    print!(
        "{}",
        serde_yaml::to_string(&crd::ForwardedService::crd()).unwrap()
    )
}

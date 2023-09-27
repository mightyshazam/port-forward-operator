extension_repo = "file://%s" % os.getcwd()
v1alpha1.extension_repo(name='my-repo', url=extension_repo)
# v1alpha1.extension(name='forwarded_service', )
target = local(
    """
    platform=`uname -p`
    if [[ $platform == 'arm' ]]; then
        echo "aarch64-unknown-linux-gnu"
    else
        echo "x86_64-unknown-linux-gnu"
    fi
    """)
cargo_config = local(
    """
    os=`uname`
    target=`uname -p`
    if [[ $os == 'Linux' ]]; then
        echo ""
    else
        if [[ $target == 'arm' ]]; then
            echo "--config target.aarch64-unknown-linux-gnu.linker=\\\"aarch64-linux-gnu-gcc\\\""
        else
            echo "--config target.x86_64-unknown-linux-gnu.linker=\\\"x86_64-unknown-linux-gnu-gcc\\\""
        fi
    fi
    """,
    env={
        'CARGO_TARGET': target
    }
)
local_resource(
    'build-controller',
    'echo $CARGO_CONFIG $CARGO_TARGET && cargo build --target $CARGO_TARGET $CARGO_CONFIG',
    env={
        'CARGO_TARGET': target,
        'CARGO_CONFIG': cargo_config
    },
    deps=['src', 'Cargo.toml', 'Cargo.lock'],
    ignore=['.github', 'README.md', 'target', 'manifests', '.gitignore'],
)

docker_context = ("target/%s/debug" % target).replace('\n', '')
docker_build(
    'docker-port-forward-operator',
    context=docker_context,
    dockerfile_contents="""
FROM ubuntu

WORKDIR /build

COPY controller .
ENTRYPOINT ["/build/controller"]
""",
    ignore=['.fingerprint', 'build', 'deps', 'examples', 'incremental'],
    only=['.'],
    match_in_env_vars=True)
k8s_yaml(kustomize('manifests/kubernetes/development'))
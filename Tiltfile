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
# docker_build('aware/spark', context='docker', dockerfile='docker/Dockerfile')
k8s_yaml(kustomize('manifests/kubernetes/development'))
# k8s_resource('csv-spark-api', port_forwards=["8080:8000"])
# k8s_resource('azurite', port_forwards=['10000:10000'])
#local_resource(
#    'create-survey-container',
#    cmd='az storage container create -n survey --connection-string ${AZURE_CONNECTION_STRING}',
#    resource_deps=['csv-spark-api'],
#    env={
#        'AZURE_CONNECTION_STRING': 'DefaultEndpointsProtocol=http;AccountName=devstoreaccount1;AccountKey=Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==;BlobEndpoint=http://127.0.0.1:10000/devstoreaccount1;QueueEndpoint=http://127.0.0.1:10001/devstoreaccount1;TableEndpoint=http://127.0.0.1:10002/devstoreaccount1;'
#    })
import dagger
from dagger import dag, function, object_type


@object_type
class Cooklatex:
    @function
    def compile(self, source: dagger.Directory) -> dagger.File:
        return (
            dag.container()
            .from_("rust:alpine")
            .with_exec(
                ["apk", "add", "--update", "--no-cache", "musl-dev", "alpine-sdk"]
            )
            .with_directory("/app", source)
            .with_workdir("/app")
            .with_exec(["cargo", "build", "--release"])
            # .with_mounted_cache("/app/target", dag.cache_volume("cooklatex"))
            .file("/app/target/release/cooklatex")
        )

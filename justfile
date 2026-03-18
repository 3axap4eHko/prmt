setup:
    git config core.hooksPath .githooks

version bump:
    cargo set-version --bump {{bump}}
    @VERSION=$(cargo pkgid | sed 's/.*@//') && \
        git add Cargo.toml Cargo.lock && \
        git commit -m "v$VERSION" && \
        git push

publish:
    @VERSION=$(cargo pkgid | sed 's/.*@//') && \
        git tag -am "v$VERSION" "v$VERSION" && \
        git push --tags && \
        cargo publish

setup:
    git config core.hooksPath .githooks

version bump:
    cargo set-version --bump {{bump}}
    @VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2) && \
        git add Cargo.toml Cargo.lock && \
        git commit -m "v$VERSION" && \
        git push

publish:
    @VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2) && \
        git tag -am "v$VERSION" "v$VERSION" && \
        git push --tags && \
        cargo publish

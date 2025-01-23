ARG APP_NAME=pp-tree-importer

################################################################################
# Build Stage
FROM rust:1.83.0-bookworm AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/bookworm-backports.list && \
    apt-get update && \
    apt-get install -y pkg-config libclang-dev libheif1/bookworm-backports libheif-dev/bookworm-backports && \
    cc --version

# Build the application.
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/importer

################################################################################
# Runtime Dependency Extractor
# Copied from https://github.com/GoogleContainerTools/distroless/issues/863
FROM debian:12 AS deb_extractor
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/bookworm-backports.list && \
    cd /tmp && \
    apt-get update && \
    apt-get download \
      libheif1/bookworm-backports \
        libaom3 libdav1d6 libde265-0 libheif-plugin-aomenc libheif-plugin-dav1d libheif-plugin-libde265 \
        libheif-plugin-x265 libnuma1 libx265-199 \
      zlib1g && \
    mkdir /dpkg && \
    for deb in *.deb; do dpkg --extract $deb /dpkg || exit 10; done

################################################################################
# Final Runtime Image
FROM gcr.io/distroless/cc-debian12:nonroot AS final

COPY --from=deb_extractor /dpkg /
COPY --from=build /bin/importer /bin/

CMD ["/bin/importer"]

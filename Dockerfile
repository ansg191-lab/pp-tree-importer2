# Setup build environment
FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm@sha256:1bd02fff3201e30948c43e6b2c9ca4c038dcf86596516cad1c7f0bc7416ffd7d AS chef
WORKDIR /app

RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/bookworm-backports.list && \
    apt-get update && \
    apt-get install -y pkg-config libclang-dev libheif1/bookworm-backports libheif-dev/bookworm-backports

# Prepare dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build Image
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release

# Runtime dependency extractor
# Copied from https://github.com/GoogleContainerTools/distroless/issues/863
FROM debian:12@sha256:0d8498a0e9e6a60011df39aab78534cfe940785e7c59d19dfae1eb53ea59babe AS deb_extractor
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/bookworm-backports.list && \
    cd /tmp && \
    apt-get update && \
    apt-get download \
      libheif1/bookworm-backports \
        libaom3 libdav1d6 libde265-0 libheif-plugin-aomenc libheif-plugin-dav1d libheif-plugin-libde265 \
        libheif-plugin-x265 libnuma1 libx265-199 \
      zlib1g && \
    mkdir -p /dpkg/var/lib/dpkg/status.d/ && \
    for deb in *.deb; do \
      package_name=$(dpkg-deb -I ${deb} | awk '/^ Package: .*$/ {print $2}'); \
      echo "Process: ${package_name}"; \
      dpkg --ctrl-tarfile $deb | tar -Oxvf - ./control > /dpkg/var/lib/dpkg/status.d/${package_name}; \
      dpkg --extract $deb /dpkg || exit 10; \
    done

# Final runtime image
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:a3c413a866ff27d0ae9e8555fd7c29991799aba085d1d7eb3348acac171a1752 AS final

COPY --from=deb_extractor /dpkg /
COPY --from=builder /app/target/release/pp-tree-importer /bin/pp-tree-importer

ENTRYPOINT [ "/bin/pp-tree-importer" ]

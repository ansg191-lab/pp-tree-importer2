# Setup build environment
FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm@sha256:af33b74dfc05452b37d6f0d27d8ebfb5f8edf88aaf3c9c84b7852e669078eaa5 AS chef
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
FROM debian:12@sha256:18023f131f52fc3ea21973cabffe0b216c60b417fd2478e94d9d59981ebba6af AS deb_extractor
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
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:db467849093a0eeea12ccb3bb0c547dd55b377bea70ffd067a16e2db305fa182 AS final

COPY --from=deb_extractor /dpkg /
COPY --from=builder /app/target/release/pp-tree-importer /bin/pp-tree-importer

ENTRYPOINT [ "/bin/pp-tree-importer" ]

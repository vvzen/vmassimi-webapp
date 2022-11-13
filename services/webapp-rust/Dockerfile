FROM centos:8

WORKDIR /app

RUN mkdir -p scripts

COPY scripts/update_centos_mirrors.sh scripts
RUN scripts/update_centos_mirrors.sh

COPY scripts/run_server.sh scripts
COPY scripts/generate_permutation.py scripts
COPY scripts/sanitize_directories.py scripts

#ENV PATH="${HOME}/.cargo/bin:${PATH}"

COPY webapp-rust-linux webapp-rust-linux

CMD ["/bin/sh", "/app/scripts/run_server.sh"]

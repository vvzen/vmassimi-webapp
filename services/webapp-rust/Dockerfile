FROM centos:8

WORKDIR /app
COPY webapp-rust-linux webapp-rust-linux
RUN mkdir -p scripts

COPY scripts/run_server.sh scripts
COPY scripts/generate_permutation.py scripts
COPY scripts/sanitize_directories.py scripts
COPY scripts/update_centos_mirrors.sh scripts

RUN scripts/update_centos_mirrors.sh && yum update -y && yum install -y tree python3

#ENV PATH="${HOME}/.cargo/bin:${PATH}"

CMD ["/bin/sh", "/app/scripts/run_server.sh"]

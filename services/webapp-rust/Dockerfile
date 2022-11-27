FROM --platform=linux/amd64 centos:8

WORKDIR /app

RUN mkdir -p scripts

COPY scripts/install_python.sh scripts
RUN scripts/install_python.sh

COPY scripts/run_server.sh scripts
COPY scripts/generate_permutation.py scripts
COPY scripts/sanitize_directories.py scripts

# Set timezone
# https://www.gnu.org/software/libc/manual/html_node/TZ-Variable.html
ENV TZ CET

#ENV PATH="${HOME}/.cargo/bin:${PATH}"

COPY webapp-rust-linux webapp-rust-linux

CMD ["/bin/sh", "/app/scripts/run_server.sh"]

FROM centos:8

WORKDIR /app
COPY webapp-rust /app/webapp-rust
COPY run_server.sh /app/run_server.sh

#ENV PATH="${HOME}/.cargo/bin:${PATH}"

CMD ["/bin/sh", "./run_server.sh"]

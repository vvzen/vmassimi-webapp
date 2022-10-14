FROM python:3.9

# Install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y

WORKDIR /app
COPY requirements.txt /app/requirements.txt
RUN python3.9 -m pip install -r requirements.txt

ENV PATH="${HOME}/.cargo/bin:${PATH}"

CMD ["python3", "src/main.py"]

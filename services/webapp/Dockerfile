FROM python:3.9

WORKDIR /src
COPY requirements.txt /src/requirements.txt
RUN python3.9 -m pip install -r requirements.txt

COPY ./app /src/app

CMD ["bash", "-c", "uvicorn app.main:app --uds /var/run/uvicorn.sock --proxy-headers --forwarded-allow-ips='*'"]
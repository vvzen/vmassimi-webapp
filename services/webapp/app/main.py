from typing import Union

from fastapi import FastAPI

app = FastAPI()

print("--> running main.py")


@app.get('/app')
def read_root():
    return {'Hello': "World"}

import os
import subprocess
from pathlib import Path

import uvicorn
from fastapi import FastAPI, Request, File, UploadFile
from fastapi.templating import Jinja2Templates
from fastapi.responses import HTMLResponse


app = FastAPI()

templates = Jinja2Templates(directory="src/static/templates", auto_reload=True)

@app.get('/app', response_class=HTMLResponse)
async def read_root(request: Request):
    return templates.TemplateResponse("index.html", {"request": request})


@app.post("/app/uploadfile")
async def create_file(file: bytes = File()):
    print("Received file: %s", file)
    return {"file_size": len(file)}


@app.post('/app/upload')
async def upload_file(upload_file: UploadFile):

    print("file: %s", upload_file.filename)
    print("content type: %s", upload_file.content_type)

    target_dir = Path("./files")
    target_dir.mkdir(mode=0o755, exists_ok=True)
    
    contents = await upload_file.read();

    with open(target_dir / "test_file", "w") as f:
        f.write(contents)

    return {"success": True}


@app.get('/app/rust')
async def test_rust(request: Request):
    p = subprocess.run(['./src/image-composite', '--version'], shell=False, capture_output=True)
    version = 'unknown'
    if p.stdout and (stdout := p.stdout.decode('utf-8')):
        version = stdout.strip().replace('image-composite', '')

    return {'version': version} 

if __name__ == '__main__':
    uvicorn.run(app='main:app', uds='/var/run/uvicorn.sock', proxy_headers=True, forwarded_allow_ips='*', workers=1)

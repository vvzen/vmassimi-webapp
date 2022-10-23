  const inputElement = document.getElementById("upload-input");
  const uploadElement = document.getElementById("submit-input");
  const unitChoice = document.getElementById("unit-choice");
  const form = document.forms.namedItem("file-upload-form");

  inputElement.addEventListener("input", handleFiles, false);
  unitChoice.addEventListener("change", handleFiles, false);
  form.addEventListener("submit", uploadFiles);

  function getBaseLog(base, y) {
    return Math.log(y) / Math.log(base);
  }

  function map_range(n, start1, stop1, start2, stop2) {
    return ((n-start1)/(stop1-start1))*(stop2-start2)+start2;
  };

  function handleFiles(){
    // Calculate total size
    let numberOfBytes = 0;
    for (const file of inputElement.files) {
      numberOfBytes += file.size;
    }
    
    if (numberOfBytes == 0){
      document.getElementById("fileSize").textContent = `0`;
      return;
    }
    // Approximate to the closest prefixed unit
    const bytesUnits = [
      "B",
      "KB",
      "MB",
      "GB",
    ];
    const bibytesUnits = [
      "B",
      "KiB",
      "MiB",
      "GiB",
    ];
    const units = unitChoice.value == "bytes" ? bytesUnits : bibytesUnits; 
    const base = unitChoice.value == "bytes" ? 1000 : 1024; 
    let exponent = Math.floor(getBaseLog(base, numberOfBytes))
    exponent = Math.min(exponent, units.length-1);
    const unitToUse = units[exponent];
    const fileSizeInUnit = numberOfBytes / Math.pow(base, exponent); 
    const fileSizeHumanReadable = fileSizeInUnit.toString().slice(0, 4);
    document.getElementById("file-size").textContent = `~${fileSizeHumanReadable} ${unitToUse}`;
    let uploadInfo = document.getElementById("upload-data-info");
    uploadInfo.style.visibility = "visible";
}

function uploadFiles(event){

  // TODO: Why do we needs these?
  event.preventDefault();
  event.stopPropagation();
  console.log("Started uploading files...");
  // Calculate total size
  const file = inputElement.files[0];
  let totalBytes = file.size;
  
  if (totalBytes <= 0) {
    console.log("Nothing to upload.")
    return;
  }
  let progressDiv = document.getElementById("upload-progress-info");
  progressDiv.style.visibility = "visible";

  let progressText = document.getElementById("upload-progress");
  let progressBar = document.getElementById("upload-progress-bar");
  let finishedSuccessDiv = document.getElementById("upload-finished-success");
  let finishedErrorDiv = document.getElementById("upload-finished-error");

  console.log("File:", file);
  console.log("totalBytes:", totalBytes);

  let formData = new FormData();
  formData.append('content-type', file.type);
  formData.append(file.name, file);

  let request = new XMLHttpRequest();

  function onError(event){
    console.error("Upload errored: ", event);  
    progressDiv.style.visibility = "hidden";
    finishedSuccessDiv.style.visibility = "collapse";
    finishedErrorDiv.style.visibility = "visible";
  }

  function onProgress(event){
    if (event.lengthComputable){
      let progress = map_range(event.loaded, 0, event.total, 0, 100);
      progressBar.ariaValueNow = progress.toString();
      progressBar.style.width = `${progress.toString()}%`;
      if (progress == 100){
        progressText.innerText = `${parseInt(progress).toString()}% - Saving the last few bits.. (this might take a while!)`;
      } 
      else {
        progressText.innerText = `${parseInt(progress).toString()}%, ${event.loaded} of ${event.total} bytes`;
      }
    }
  }

  function onUploadFinished(event){
    progressDiv.style.visibility = "hidden";
    finishedSuccessDiv.style.visibility = "visible";
  }

  request.upload.addEventListener("error", onError);
  request.upload.addEventListener("progress", onProgress);

  //request.upload.addEventListener("readystatechange", onReadyStateChange);
  request.upload.addEventListener("load", onUploadFinished);
  let endpoint = "/app/api/upload-archive";
  //let endpoint = "http://localhost:3000/upload";
  console.log("Uploading files to", endpoint);
  request.open("POST", endpoint);
  request.send(formData);
}

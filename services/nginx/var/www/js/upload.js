const inputElement = document.getElementById("upload-input");
const uploadElement = document.getElementById("submit-input");
const unitChoice = document.getElementById("unit-choice");
const form = document.forms.namedItem("file-upload-form");

const progressDiv = document.getElementById("upload-progress-info");
const progressText = document.getElementById("upload-progress");
const progressBar = document.getElementById("upload-progress-bar");
const finishedSuccessDiv = document.getElementById("upload-finished-success");
const finishedErrorDiv = document.getElementById("upload-finished-error");
const uploadErrorMessage = document.getElementById("upload-error-message");

const EXPECTED_MIME_TYPE = 'application/x-gzip';

// Entry points
// -----------------------------------------------------------------------------
inputElement.addEventListener("input", handleFiles, false);
unitChoice.addEventListener("change", handleFiles, false);
form.addEventListener("submit", uploadFiles);

// Functions
// -----------------------------------------------------------------------------
function getBaseLog(base, y) {
  return Math.log(y) / Math.log(base);
}

function mapRange(n, start1, stop1, start2, stop2) {
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

function onBeforeUnload(event){
  // Cancel the event
  event.preventDefault();
  let message = "Are you sure you want to exit? You will lose the current progress on the upload!";
  event.returnValue = message; 
  return message;
}

function sanityCheck(file){
  
  if (!file.name.endsWith(".tar.gz")){
    let errorMessage = `Il file scelto (${file.name}) non finisce con .tar.gz. Sei sicuro sia un archivio TAR GZ?`; 
    console.error(file);
    console.error(errorMessage);
    progressDiv.style.visibility = "hidden";
    finishedSuccessDiv.style.visibility = "collapse";
    finishedErrorDiv.style.visibility = "visible";
    uploadErrorMessage.innerHTML = errorMessage;
    return false;
  }

  if (file.type != (EXPECTED_MIME_TYPE)){
    let errorMessage = `Il MIME-type del file che hai scelto (${file.name}) non è '${EXPECTED_MIME_TYPE}'. Sei sicuro che sia un archivio TAR GZIP?`; 
    console.error(file);
    console.error(errorMessage);  
    progressDiv.style.visibility = "hidden";
    finishedSuccessDiv.style.visibility = "collapse";
    finishedErrorDiv.style.visibility = "visible";
    uploadErrorMessage.innerHTML = errorMessage;
    return false;
  }

  return true;
}

function uploadFiles(event){
  
  // https://developer.mozilla.org/en-US/docs/Web/API/Window/beforeunload_event
  window.addEventListener('beforeunload', onBeforeUnload);

  // TODO: Why do we needs these?
  event.preventDefault();
  event.stopPropagation();
  console.log("Started uploading files...");

  // Calculate total size
  const file = inputElement.files[0];

  // If the archive doesn't look like it's an actual tar.gz archive, tell him!
  if (!sanityCheck(file)){
    return;
  }

  let totalBytes = file.size;
  
  if (totalBytes <= 0) {
    console.log("Nothing to upload.")
    return;
  }
  progressDiv.style.visibility = "visible";

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
    window.removeEventListener('beforeunload', onBeforeUnload);
  }

  function onProgress(event){
    if (event.lengthComputable){
      let progress = mapRange(event.loaded, 0, event.total, 0, 100);
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
    window.removeEventListener('beforeunload', onBeforeUnload);
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


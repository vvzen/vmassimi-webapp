const generateButton = document.getElementById("generate-preview-button");
generateButton.addEventListener("click", onGeneratePreview);

const imageDiv = document.getElementById("generated-image-container");
imageDiv.style.display = "none";
const img = document.getElementById("generated-image");
img.style.maxWidth = "512px";

let numAttempts = 0;
let getJobInfoIntervalID;

function onGeneratePreview(){
  console.log("Generating Random Cat preview...");  

  let options = {
    method: 'POST',
  }

  let url = `${window.location.origin}/app/api/random`;

  // Ask to generate a random image
  fetch(url, options)
    .then((response) => response.json())
    .then((data) => {

      console.log(data);

      // This is the URL where we will retrieve the image, once it has been rendered
      if (data.endpoint && data.job_id) {
        let job_url = `${window.location.origin}/app/${data.endpoint}?job_id=${data.job_id}`; 
        console.log("job_url:", job_url);

        // Keep trying until we get a positive response, or we run out of time/attempts
        getJobInfoIntervalID = setInterval(getJobInfo, 10 * 1000, job_url);

      }
      else {
        // TODO: show a user error message
        console.error(`Something went wrong with the POST request to ${url}`);
      }

    })
    .catch((error) => {
      console.error("Failed to retrive image from API endpoint:", error);
    });
}

function getJobInfo(url){

  console.log("getJobInfo for url:", url);

  if (numAttempts > 20){
    // TODO: Show the error to the user
    console.error("Went over the max number of attempts to retrieve job info.");
    clearInterval(getJobInfoIntervalID);
  }

  let options = {
    method: 'GET'
  }

  fetch(url, options)
    .then((response) => response.json())
    .then((data) => {
      console.log("Job result", data);

      if (data.status == "FAILED"){
        console.error("Job has failed.");
        clearInterval(getJobInfoIntervalID);
      }
      else if (data.status == "STARTED"){
        console.info("Job is in progress.");
      }
      else if (data.status == "COMPLETED"){
        console.info("Job has completed.");
        img.src = `data:image/png;base64,${data.image}`;
        img.style.display = "block";
        imageDiv.style.display = "block";
        clearInterval(getJobInfoIntervalID);
      }
    })
    .catch((error) => {
      console.error(`Failed to retrieve Job from ${url} endpoint:`, error);
      img.style.display = "block";
    });

  numAttempts++;
  console.log(numAttempts);
}

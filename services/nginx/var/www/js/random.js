const JOB_RETRIEVAL_INTERVAL = 5 * 1000; // ms

// Generate
const generateButton = document.getElementById("generate-preview-button");
generateButton.addEventListener("click", onGeneratePreview);

// Report progress
const progressDiv = document.getElementById("job-progress-info");
const progressBar = document.getElementById("job-progress-bar");
const progressText = document.getElementById("job-progress-report");
const recipeParagraph = document.getElementById("job-recipe");
const progressRegex = /PROGRESS: (\d{2})%;([ :.\w\d]*)/;

const imageDiv = document.getElementById("generated-image-container");
imageDiv.style.display = "none";
const img = document.getElementById("generated-image");
img.style.maxWidth = "512px";

// Make this into a hashmap<jobId: string, numAttempts: u64>
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
        getJobInfoIntervalID = setInterval(getJobInfo, JOB_RETRIEVAL_INTERVAL, job_url);
        progressDiv.style.visibility = "visible";
        progressText.innerText = "Just started!";
        //generateButton.style.visibility = "hidden"
        generateButton.style.display = "none";

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

  if (numAttempts > 50){
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

      if (data.recipe){
        let recipe = data.recipe.split("\n").join("\n\n");
        recipeParagraph.innerText = data.recipe;
      }
      else {
        recipeParagraph.innerText = "?";
      }

      if (data.status == "FAILED"){
        console.error("Job has failed.");
        clearInterval(getJobInfoIntervalID);
        //progressDiv.style.visibility = "hidden";
        progressDiv.style.display = "none";
      }
      else if (data.status == "STARTED"){

        console.log("Job is in progress.");
        console.log(data.progress);

        progressDiv.style.display = "block";

        // Extract the percentage
        let progressString = data.progress;
        //progressDiv.style.visibility = "visible";

        let results = progressRegex.exec(data.progress);
        let progress = results[1];
        let info = results[2];

        console.log("Results:", results);
        console.log("progress:", progress);
        console.log("info:", info);

        progressBar.ariaValueNow = progress;
        progressBar.style.width = `${progress}%`;
        progressText.innerText = `${info}`;

      }
      else if (data.status == "COMPLETED"){
        console.log("Job has completed.");
        img.src = `data:image/png;base64,${data.image}`;
        img.style.display = "block";
        imageDiv.style.display = "block";
        //generateButton.style.visibility = "visible"
        generateButton.style.display = "inline-block";

        //progressDiv.style.visibility = "hidden";
        progressDiv.style.display = "none";

        clearInterval(getJobInfoIntervalID);
        numAttempts = 0;
      }
    })
    .catch((error) => {
      console.error(`Failed to retrieve Job from ${url} endpoint:`, error);
      img.style.display = "block";
    });

  numAttempts++;
  console.log(numAttempts);
}
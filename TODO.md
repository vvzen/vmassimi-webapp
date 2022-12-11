# TODO

## Permutation algo

- [x] Skin selection
- [x] Mouth 06 -> force Hands 06 constraint
- [x] Eyelashes constraint based on chosen eye
  - We already do this since there is a eyelashes_no directory ?
- [x] Eyes 12 -> skip Mouth 4,5 constraint
- [ ] Ears_2 -> no head gadgets constraint
- [ ] Constrained choices based on initial Head gadget choice
- [ ] No lazer if any of wool, cop, hat, weed, rus

## Infrastructure

- [x] [Frontend] Show image when hovering over file node in Inventory
- [x] [Backend] Add /api/status endpoint
- [x] [Frontend/Backend] Add 'Random' Page to generate a random cat
  - [x] [Frontend] It should have a button that says something like: 'Generate random cat'
  - [ ] [Frontend] It should clearly show the current version of the archive being used
  - [x] [Backend] Add REST API to generate 1 image based on a random recipe
    - [x] /api/v1/random Endpoint. Returns back the job id submitted
    - [x] /api/v1/jobs?job_id=my_id Endpoint. Returns back data of a job
  - [ ] [Backend] Add REST API to generate 1 image based on an input recipe provided in POST request body
    - [ ] /api/v1/generate Endpoint. Returns back a base64 image in the response body
- [ ] [Frontend/Backend] Add 'Cart' Page
  - [ ] Here you can generate a preview imaged based on the current nodes in the Inventory

- [Frontend/Backend] While the archive is being sanitized, the user shouldn't be able to click on the generate Random cat

# README

The idea is - as always - to be inspired by the UNIX design and philosophy.
This means creating a suite of tools that work together, instead of creating a single bulky monolithic app that does it all. Elegance means using the right tool for the right job.
(eeeeerm which is what [https://www.youtube.com/watch?v=AaCgydeMu64&t=59](https://www.youtube.com/watch?v=AaCgydeMu64&t=59]) does since he probably only knows nodejs.. ?)

The first step is to generate a recipe that lists the images to combine.
This can be done in python, and is basically a way to generate all of the potential permutations.

Then, the actual assembly can be done via something else.
I was originally thinking OpenImageIO, but probably it's way easier to write something in OpenFrameworks or even in Nannou.


## Current setup

1. Data preparation and cleaning 

Untar the .tar.gz in target directory.
Run the sanitization script to ensure there are no spaces, etc.
This will generate a `./operations.sh` script that will perform the actual renames of the directories and files.

```bash
./sanitize_directories.py /path/to/root/sphynx_program_v001
./operations.sh
```

2. Generate a single permutation using the `generate_permutations.py` script.

This will spit out to stdout the 'recipe' of the images to overlay together. The recipe currently looks like this:
```
root_dir: ~/some/path/input_for_sphynx
01_background/01_common_background/Background_C_20.png
02_body_skins/Body_Skin_Standard_pink.png
03_ears/ear_1/01_ear_1_skins/Ear_1_Psy_cyan.png
04_body_lines/Body_LINES_4k.png
05_eyes/eyes_7/01_eyes_7_colors/eyes_7_uncommon_colors/02_eyes_7_pupils/Eyes_7_pupilla_1.png
06_mouths/mouth_1/01_mouth_1/01_mouth_1_skins/Mouth_1_Psy_green.png
07_hands/hand_5/02_hand_5_lines/Hand_5_Line.png
```

3. Feed the recipe file to the Rust app. The app will overlay all images together and spit out the result on disk.

`$ cat my_recipe_file | ./image-composite/target/release/image-composite --image-name my_name`


## TODO

1. Support for 'streams'. Some images can only go with some other images.
The way to achieve that will be through tagging them to be used in specific 'streams'.
EG: `Body_Skin_Tiger_zebra.png`. The stream is `Skin_Tiger`. This means that this image can only go together with images of the same `Skin_Tiger` stream.

2. Generation of the JSON metadata files.
This can turn out to be a PITA since we might need to already have the 'address' of the NFTs in order add them to the Metadata, which means this needs to be coordinated with the upload on OpenSea, etc.

## Performance considerations

Current time per single iteration (generation of path + rendering) : ~3.6 seconds
(3,6 sec * 10000 iterations) / 60 / 60 = 10 hours

If we manage to parallelize the pipeline, and run 5 processes at once, we can maybe cut this down to 2 hours total.

### Further considerations (2022/02/28)

The thing that it's currently slow and can be easily parallelized is the generation of the image.
So the idea could be to first generate a massive recipe file that contains instructions on how to overlay 10'000 images. Then we, can split this file in N batches, iterate through every batch and run the Rust app.


## How many combinations?

(20 + 3 + 8 + 8) backgrounds * 25 skins * ((10 + 9) * 7) eyes * 6 mouths * 12 hands

39 backgrounds * 25 skins * 133 eyes * 6 mouths * 12 hands

-> 9'336'600

This means that the chances of a collision are 1 / 9'336'600


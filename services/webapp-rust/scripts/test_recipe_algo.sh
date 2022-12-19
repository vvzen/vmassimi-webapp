#!/bin/bash

input_dir=/Users/valerioviperino/dev/personal/vmassimi/sample-files/sphynx_program/programm

recipe=$(python3 generate_permutation.py $input_dir)

image_app=~/dev/personal/vmassimi/vmassimi-tools/scripts/image-composite/image-composite-macos
recipe_checksum=$(echo $recipe | sha256sum | cut -c 8)
image_name="image_$recipe_checksum"

echo $recipe | $image_app --image-name $image_name

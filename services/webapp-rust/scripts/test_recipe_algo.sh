#!/bin/bash

set -e

input_dir=/Users/valerioviperino/dev/personal/vmassimi/sample-files/sphynx_program/programm
image_app=~/dev/personal/vmassimi/vmassimi-tools/scripts/image-composite/image-composite-macos

#recipe=$(python3 generate_permutation.py $input_dir)
python3 generate_permutation.py $input_dir 1>tmp_recipe 2>tmp_recipe_stderr
recipe_checksum=$(cat tmp_recipe | md5sum | awk '{print $1}' | cut -c-8)
image_name="image_$recipe_checksum"

echo $image_name

cat tmp_recipe | $image_app --image-name $image_name

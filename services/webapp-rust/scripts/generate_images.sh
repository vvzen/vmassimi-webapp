#!/bin/bash

set -e

# Path pointing to the compiled Rust app
#image_app="/app/image-composite"
image_app=/Users/valerioviperino/dev/personal/vmassimi/vmassimi-tools/scripts/image-composite/image-composite-macos

# Path pointing to the first directory containing all of the dir structure
#input_directory="../input_for_sphynx"
input_directory=~/dev/personal/vmassimi/sample-files/sphynx_program

# Sanity checking
if [ ! -d "$input_directory" ]; then
    echo "Input directory wasn't found on disk: $input_directory"
    echo "Exiting.."
    exit 1
fi

if [ ! -f "$image_app" ]; then
    echo "The Compiled Rust app wasn't found here: $image_app"
    echo "Please fix this error. Exiting.."
    exit 1
fi

if [ ! -x "$image_app" ]; then
    echo "The Compiled Rust app ($image_app) cannot be executed."
    echo "Please update the permissions. Exiting.."
    exit 1
fi

total_iterations=100

# NB: this is a bash built-in
SECONDS=0

echo "--> Generating permutations.."
mkdir -p recipes
for iter_num in $(seq $total_iterations); do

    echo "Generating permutation $iter_num / $total_iterations"
    recipe_file="recipes/permutation_${iter_num}"

    touch recipe_std_err
    python3 generate_permutation.py $input_directory 1> $recipe_file 2> recipe_std_err
    echo -e "$iter_num\n$(cat $recipe_file)\n$(cat recipe_std_err)\n" >> recipes/recipes_log

    echo "Rendering image.."
    eval "cat $recipe_file | $image_app --image-name test_${iter_num}" 
done
rm recipe_std_err

duration=$SECONDS
echo "$(($duration / 60)) minutes and $(($duration % 60)) seconds elapsed."
echo "This means an average of $(($duration / $total_iterations)) seconds per iteration."

disk_space=$(du -sh "./output")
echo "Space occupied: $disk_space"

echo "--> All done."

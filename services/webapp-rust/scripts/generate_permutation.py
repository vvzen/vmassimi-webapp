#!/usr/bin/env python3
import os
import re
import sys
import bisect
import random
import hashlib

OVERLAY_REGEX = re.compile(r"^\d{2}")
STREAM_REGEX = re.compile(r"(?:^(?:Body_Skin_)|^(?:\w+_\d+_))([&]*[A-Za-z0-9&_]*)\.png")
CURRENT_STREAM = None
SKINS_DIR_NAME = "02_body_skins"
TREES_FILE_NAME = "trees.txt"

# NB: As usual UNIX practice, stderr is used for logging/errors
# stdout is used to generate the output we're interesting in
# this makes the piping easy

def get_probability(name):

    name = name.lower()

    tokens = name.split("_")
    for token in tokens:
        if token == "uncommon":
            return 15.0 / 100.0
        elif token == "epic":
            return 7.0 / 100.0
        elif token == "legendary":
            return 3.0 / 100.0

    # Common
    return 75.0 / 100.0


def pick_leaf(leaves):
    if not leaves:
        raise RuntimeError("Cannot pick a variant - "
                           "no variants were provided!")

    index = random.randint(0, len(leaves)-1)
    return leaves[index]


def pick_variant(variants):

    if not variants:
        raise RuntimeError("Cannot pick a variant - "
                           "no variants were provided!")

    probabilities = []

    for variant in variants:
        probabilities.append((variant, get_probability(variant)))

    probabilities = sorted(probabilities, key=lambda e: e[1])

    dice = random.randrange(0.0, 100.0) / 100.0
    for variant_name, variant_prob in probabilities:
        if dice <= variant_prob:
            return variant_name

    return probabilities[-1][0]

def get_stream(file_name):
    match = STREAM_REGEX.match(file_name)
    if not match:
        return ""

    groups = match.groups()
    if not groups:
        return ""

    return groups[0]

# TODO: once a combination has been chosen, it can't be chosen again!

def traverse(tree, branch, root_dir):

    global CURRENT_STREAM

    sys.stderr.write("Looking into %s\n" % root_dir)
    branch.append(os.path.basename(root_dir))
    sys.stderr.write("\tCurrent branch: %s\n" % branch)

    disk_content = os.listdir(root_dir)

    # 1. Is this is a directory with overlays?
    overlays = [
        d for d in disk_content
        if OVERLAY_REGEX.match(d) and os.path.isdir(os.path.join(root_dir, d))
    ]
    overlays = sorted(overlays)
    if overlays:
        sys.stderr.write("\tFound overlays: %s\n" % overlays)
        for overlay in overlays:
            current_branch = branch[:]
            traverse(tree, branch, os.path.join(root_dir, overlay))
            branch = current_branch

        return

    # 2. Is this a directory with variants?
    variants = [
        d for d in disk_content
        if os.path.isdir(os.path.join(root_dir, d))
    ]

    if variants:
        chosen_variant = pick_variant(variants)
        sys.stderr.write(f"\tChosen variant: {chosen_variant}\n")
        traverse(tree, branch, os.path.join(root_dir, chosen_variant))

        return

    # 3. Is this a directory with the final leaves?
    sys.stderr.write("\tNo variants, looking for leaves..\n")
    all_leaves = [
        f for f in disk_content
        if os.path.isfile(os.path.join(root_dir, f))
        and f not in [".DS_Store"]
    ]
    potential_leaves = []
    if CURRENT_STREAM and "skins" in branch[-1]:
        for leaf in all_leaves:
            match = STREAM_REGEX.match(leaf)
            if not match or not match.groups():
                continue
            if match and CURRENT_STREAM == match.groups()[0]:
                potential_leaves.append(leaf)
        sys.stderr.write("\tPotential leaves (after filtering "
                         "for stream): %s\n" % potential_leaves)
    else:
        potential_leaves = all_leaves[:]
        sys.stderr.write("\tPotential leaves (no filtering): %s\n"
                         % potential_leaves)

    # This shouldn't happen (every directory should contain something in the
    # end) - but still
    if not potential_leaves:
        return

    final_leaf = pick_leaf(potential_leaves)

    # Pick the skin / stream
    if branch[-1] == SKINS_DIR_NAME:
        CURRENT_STREAM = get_stream(final_leaf)
        sys.stderr.write(f"\tCurrent stream: %s\n" % CURRENT_STREAM)

    branch.append(final_leaf)
    sys.stderr.write(f"\tFound final leaf ({final_leaf}). \n")

    tree.append("/".join(branch))
    return tree


def generate_permutation(root_dir):

    tree = []
    current_branch = []
    traverse(tree, current_branch, root_dir)

    sys.stdout.write("root_dir: %s\n" % root_dir)
    root_name = os.path.basename(root_dir)

    md5 = hashlib.md5()

    for branch in tree:
        branch_cleaned = branch[len(root_name)+1:]
        sys.stdout.write(branch_cleaned)
        sys.stdout.write("\n")
        md5.update(branch_cleaned.encode("utf-8"))

    sys.stderr.write("\n")
    return md5.hexdigest()


def main():
    if len(sys.argv) < 2:
        sys.stderr.write("No root paths provided. Nothing to do. Exiting..\n")
        sys.exit(1)

    root_dir = sys.argv[1]
    if not os.path.exists(root_dir):
        sys.stderr.write(f"Root path {root_dir} doesn't exist. Exiting..\n")
        sys.exit(1)

    if not os.path.exists(TREES_FILE_NAME):
        with open(TREES_FILE_NAME, "w") as f:
            f.write("")

    root_dir = os.path.abspath(root_dir)
    checksum = generate_permutation(root_dir)

    # Check if the checksum is on disk already. The chances are quite low
    # (1 over 9336600) so that's why we use this 'primitive' mechanism,
    # instead of avoiding altogether the creating of a permutation that might
    # have been already generated
    f = open(TREES_FILE_NAME, "r")
    checksums_list = f.read().splitlines()
    f.close()

    # We use binary search here O(log N) to avoid
    # looping through every element of the list O(N)
    index = bisect.bisect_left(checksums_list, checksum)
    while index != len(checksums_list) and checksums_list[index] == checksum:
        checksum = generate_permutation(root_dir)
        index = bisect.bisect_left(checksums_list, checksum)

    sys.stderr.write("Permutation completed. Checksum: %s\n" % checksum)

    # Write the new checksum on disk (keep the list sorted when appending)
    checksums_list.insert(index, checksum)
    with open(TREES_FILE_NAME, "w") as f:
        f.write("\n".join(checksums_list))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import os
import re
import sys
import bisect
import random
import hashlib
from typing import Any

OVERLAY_REGEX = re.compile(r"^\d{2}")
SKIN_STREAM_REGEX = re.compile(r"(?:^(?:body_skin_)|^(?:\w+_\d+_))([&]*[A-Za-z0-9&_]*)\.png")
SKIN_STREAM = None
SKINS_DIR_NAME = "02_body_skins"
TREES_FILE_NAME = "trees.txt"

# Spaghetti code stuff to handle the various special cases
MOUTH_06_HAS_BEEN_CHOSEN = False
MOUTH_06_HAS_GADGETS = False
MOUTH_06_REGEX = re.compile(r"mouth_\d*6")
MOUTH_06_HAS_GADGET_REGEX = re.compile(r"mouth_\d*6_gadget")

MOUTH_04_05_REGEX = re.compile(r"mouth_0[45]")
EYES_12_REGEX = re.compile(r"eyes_12")
EYES_12_HAS_BEEN_CHOSEN = False

DEBUG_LEVEL = int(os.getenv("DEBUG_LEVEL", 0))

# NB: As usual UNIX practice, stderr is used for logging/errors
# stdout is used to generate the output we're interesting in
# this makes the piping easy

NAMES_PROBABILITIES = {
    "common": 75.0 / 100.0,
    "uncommon": 15.0 / 100.0,
    "epic": 7.0 / 100.0,
    "legendary": 3.0 / 100.0,
}


def get_probability(name: str) -> float:
    """Given a name in input, print back the probability associated with that name being chosen

    :param name: name
    :type name: str

    :return: probability of that name being chosen
    :rtype: float
    """

    name = name.lower()
    tokens = name.split("_")

    for token in tokens:
        probability = NAMES_PROBABILITIES.get(token, None)
        if probability:
            return probability

    return NAMES_PROBABILITIES["common"]


def pick_leaf(leaves: list) -> Any:
    """Choose a random element from the given ones

    :param leaves: list to chose elements from
    :type leaves: list[str]

    :raises RuntimeError: if no variants were provided

    :return: an element chosen at random
    :rtype: str
    """

    if not leaves:
        raise RuntimeError("Cannot pick a leaf - "
                           "no leafs were provided!")

    # Special Case: mouths 06 can only go with hands 06
    if MOUTH_06_HAS_BEEN_CHOSEN:
        # If we are choosing hands 06, then we need to apply
        # some special logic. Otherwise we continue as usual
        we_are_choosing_hands = [leaf for leaf in leaves if 'hand' in leaf]
        if we_are_choosing_hands:
            return pick_hand_06_leaf(leaves)

    index = random.randint(0, len(leaves)-1)
    return leaves[index]


def pick_hand_06_leaf(leaves: list):

    if DEBUG_LEVEL >= 1:
        sys.stderr.write("\tPicking hand leaf constrained by mouth 06 choice\n")

    new_leaves = []
    for leaf in leaves:

        # If the mouth 06 already has gadgets,
        # we don't want to pick any more gadgets here
        if MOUTH_06_HAS_GADGETS and leaf.startswith('gadget'):
            if DEBUG_LEVEL >= 1:
                sys.stderr.write(f"\tSkipping hand leaf {leaf} since it has gadgets\n")
            continue

        if '6' in leaf:
            new_leaves.append(leaf)

    index = random.randint(0, len(leaves)-1)
    return leaves[index]


def should_use_weighted_approach(variants):
    return len([v for v in variants if 'uncommon' in v]) != 0


def pick_variant_weighted_approach(variants):

    if DEBUG_LEVEL >= 1:
        sys.stderr.write("\tPicking variant based on weighted approach\n")

    probabilities = []

    for variant in variants:
        probabilities.append((variant, get_probability(variant)))

    probabilities = sorted(probabilities, key=lambda e: e[1])

    # Throw a dice with a 100
    dice = random.randrange(0.0, 100.0) // 100.0

    for variant_name, variant_prob in probabilities:
        if dice <= variant_prob:
            return variant_name
    return probabilities[-1][0]


def pick_variant_random_approach(variants):

    if DEBUG_LEVEL >= 1:
        sys.stderr.write(f"\tPicking random variant from {variants}\n")

    num_variants = len(variants)
    if num_variants == 1:
        return variants[0]

    if num_variants == 0:
        raise RuntimeError("Cannot pick a variant - "
                           "no variants were provided!")

    assert num_variants-1 > 0

    index = random.randint(0, num_variants-1)
    return variants[index]


def pick_variant(variants):

    global MOUTH_06_HAS_BEEN_CHOSEN

    if not variants:
        raise RuntimeError("Cannot pick a variant - "
                           "no variants were provided!")

    if DEBUG_LEVEL >= 1:
        sys.stderr.write(f"\tPicking variant. Initial variants: {variants}\n")

    # Legendary/Uncommon items: compute the weighted probability of a variant
    # based on its name, then pick it
    if should_use_weighted_approach(variants):
        return pick_variant_weighted_approach(variants)

    # Mouth 06 case
    # If we are choosing hands, and mouth 06 has been chosen, we can only use hands 06
    if MOUTH_06_HAS_BEEN_CHOSEN:
        we_are_choosing_hands = [v for v in variants if 'hand' in v]
        if we_are_choosing_hands:
            variants = [v2 for v2 in variants if '6' in v2]

    # Mouth 04 and 05 case: they can't go with Eyes 12
    if EYES_12_HAS_BEEN_CHOSEN:
        variants = [v for v in variants if not MOUTH_04_05_REGEX.match(v)]

    # Normal case: equal probabilities (just chose one at random)
    return pick_variant_random_approach(variants)


def get_skin_stream(file_name: str) -> str:
    match = SKIN_STREAM_REGEX.match(file_name)
    if not match:
        return ""

    groups = match.groups()
    if not groups:
        return ""

    return groups[0]

# TODO: once a combination has been chosen, it can't be chosen again!


def traverse(tree: list, branch: list, parent_dir: str):

    # TODO: Find a way that doesn't rely on globals
    global SKIN_STREAM
    global EYES_12_HAS_BEEN_CHOSEN
    global MOUTH_06_HAS_BEEN_CHOSEN
    global MOUTH_06_HAS_GADGETS

    if DEBUG_LEVEL >= 2:
        sys.stderr.write("Looking into %s\n" % parent_dir)
    branch.append(os.path.basename(parent_dir))

    if DEBUG_LEVEL >= 2:
        sys.stderr.write("\tCurrent branch: %s\n" % " / ".join(branch))

    disk_content = os.listdir(parent_dir)

    # 1. Is this is a directory with overlays?
    # EG: A directory with final files
    overlays = [
        d for d in disk_content
        if OVERLAY_REGEX.match(d) and os.path.isdir(os.path.join(parent_dir, d))
    ]
    overlays = sorted(overlays)
    if overlays:
        if DEBUG_LEVEL >= 2:
            sys.stderr.write("\tFound overlays: %s\n" % overlays)

        for overlay in overlays:
            current_branch = branch[:]
            traverse(tree, branch, os.path.join(parent_dir, overlay))
            branch = current_branch

        # End of recursion
        return

    # 2. Is this a directory with variants?
    # EG: a directory containing other directories, each one with leaves
    variants = sorted([
        d for d in disk_content
        if os.path.isdir(os.path.join(parent_dir, d))
    ])

    if variants:
        chosen_variant = pick_variant(variants)
        if DEBUG_LEVEL >= 1:
            sys.stderr.write(f"\tChosen variant: '{chosen_variant}'\n")
        traverse(tree, branch, os.path.join(parent_dir, chosen_variant))

        return

    # 3. Is this a directory with the final leaves?
    if DEBUG_LEVEL >= 2:
        sys.stderr.write("\tNo variants, looking for leaves..\n")
    all_leaves = [
        f for f in disk_content
        if os.path.isfile(os.path.join(parent_dir, f))
        and f not in [".DS_Store"]
    ]
    potential_leaves = []
    if SKIN_STREAM and "skins" in branch[-1]:
        for leaf in all_leaves:
            match = SKIN_STREAM_REGEX.match(leaf)
            if not match or not match.groups():
                continue
            if match and SKIN_STREAM == match.groups()[0]:
                potential_leaves.append(leaf)

        if DEBUG_LEVEL >= 2:
            sys.stderr.write("\tPotential leaves (after filtering "
                             "for stream): %s\n" % potential_leaves)
    else:
        potential_leaves = all_leaves[:]

        if DEBUG_LEVEL >= 2:
            sys.stderr.write("\tPotential leaves (no filtering): %s\n"
                             % potential_leaves)

    # This shouldn't happen (every directory should contain something in the
    # end) - but still
    if not potential_leaves:
        return

    final_leaf = pick_leaf(potential_leaves)

    # Pick the skin / stream
    if branch[-1].lower() == SKINS_DIR_NAME:
        SKIN_STREAM = get_skin_stream(final_leaf)

        if DEBUG_LEVEL >= 1:
            sys.stderr.write(f"\t*** Chosen stream: '{SKIN_STREAM}'\n")

    # Are we in the special case of mouths 04/05 ?
    if EYES_12_REGEX.match(final_leaf):
        EYES_12_HAS_BEEN_CHOSEN = True
        if DEBUG_LEVEL >= 1:
            sys.stderr.write("\t--> Chosen Eyes 12. Special behaviour will be activated.\n")

    # Are we in the special case of mouths/hands 06 ?
    if MOUTH_06_REGEX.match(final_leaf):
        MOUTH_06_HAS_BEEN_CHOSEN = True
        if DEBUG_LEVEL >= 1:
            sys.stderr.write("\t--> Chosen mouth 06. Special behaviour will be activated.\n")

    if MOUTH_06_HAS_GADGET_REGEX.match(final_leaf):
        MOUTH_06_HAS_GADGETS = True
        if DEBUG_LEVEL >= 1:
            sys.stderr.write("\t--> mouth 06 has gadgets. No gadgets should appear in 'hands'\n")

    branch.append(final_leaf)

    if DEBUG_LEVEL >= 1:
        sys.stderr.write(f"\t<^> Found final leaf ({final_leaf}). \n")

    tree.append("/".join(branch))
    return tree


def generate_permutation(root_dir: str) -> str:
    """Generate a random permutation, and return the checksum describing it.

    :param root_dir: absolute path to the start of the archive
    :type root_dir: str

    :return: md5sum checksum of the generated tree
    :rtype: str
    """

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
    # USAGE:
    # ./generate_permutations.py /some/path/to/sphynx_program/program

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

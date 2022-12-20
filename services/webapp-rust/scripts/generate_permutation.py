#!/usr/bin/env python3
import os
import re
import sys
import bisect
import random
import fnmatch
import hashlib
from typing import Any, List

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

HANDS_07_HAS_BEEN_CHOSEN = False

EARS_2_HAS_BEEN_CHOSEN = False
EARS_2_REGEX = re.compile(r"ear_2")

EYES_NUM_CHOSEN = None
EYES_REGEX = re.compile(r"(eyes_\d+)")

MOUTH_04_05_REGEX = re.compile(r"mouth_0[45]")
EYES_12_HAS_BEEN_CHOSEN = False

HEAD_GADGET_CHOSEN: str = ""

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

# Special variables for the head
# Expressed with 'fnmatch' syntax
HEAD_GAGETS_COMBINATIONS = {
    'head_gadget_alloro': {
        'y': ['*gadget_ears*'],
        'n': ['*gadget_earrings*'],
    },
    'head_gadget_wool': {
        'y': ['*gadget_ears*'],
        'n': ['*gadget_earrings*'],
    },
    'head_gadget_cop': {
        'y': ['*gadget_ears*'],
        'n': ['*gadget_earrings*'],
    },
    'head_gadget_devil': {
        'n': ['*gadget_ears*'],
        'y': ['*gadget_earrings*'],
    },
    'head_gadget_hat': {
        'n': ['*peace*', '*cross*', '*feather*'],
        'y': ['*gadget_earrings*'],
    },
    'head_gadget_hat1': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_hat2': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_pika': {
        'n': ['*gadget_ears*'],
        'y': ['*gadget_earrings*'],
    },
    'head_gadget_angel': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_sail': {
        'n': ['*peace*', '*cross*', '*feather*'],
        'y': [],
    },
    'head_gadget_weed': {
        'n': ['*peace*', '*cross*', '*gadget_earrings*'],
        'y': [],
    },
    'head_gadget_crown': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_party': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_rus': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_bomb': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_hood': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_punk': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
    'head_gadget_bandana': {
        'y': [],
        'n': ['*gadget_earrings*', '*peace*', '*feather*', '*cross*'],
    },
    'head_gadget_bandana1': {
        'y': ['*gadget_ears*'],
        'n': ['*gadget_earrings*'],
    },
    'head_gadget_bandana2': {
        'y': ['*gadget_ears*'],
        'n': ['*gadget_earrings*'],
    },
    'head_gadget_beer': {
        'y': ['*gadget_ears*', '*gadget_earrings*'],
        'n': [],
    },
}

SKINS_TO_VALID_MOUTHS_MAP = {
    'silver': [1, 2, 3],
    'alien_cheeta': [1, 2, 3, 6, 7],
    'dark_pink': [1, 2, 7],
    'gold': [1, 2, 3, 7],
    'tiger_cheetah': [1, 2, 3, 7],
    'tiger_giraffe': [1, 2, 3, 7],
    'tiger_tiger': [1, 2, 3, 7],
    'tiger_grey': [1, 2, 3, 7],
    'tiger_zebra': [1, 2, 3, 7],
    'zombie': [1, 2, 7],
}


def print_debug_message_once(message: str):
    if not getattr(print_debug_message_once, '_data', None):
        print_debug_message_once._data = {}

    data = getattr(print_debug_message_once, '_data')

    if message not in data:
        sys.stderr.write(message)
        print_debug_message_once._data[message] = 1


def get_probability(name: str) -> float:
    """Given a name in input, return back the probability associated with that name being chosen

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

    if DEBUG_LEVEL >= 1:
        sys.stderr.write(f"\tPicking leaf. Initial choices: {leaves}\n")

    # Special Case: mouths 06 can only go with hands 06
    if MOUTH_06_HAS_BEEN_CHOSEN:
        # If we are choosing hands 06, then we need to apply
        # some special logic. Otherwise we continue as usual
        we_are_choosing_hands = [leaf for leaf in leaves if 'hand' in leaf]
        if we_are_choosing_hands:
            return pick_hand_06_leaf(leaves)

    # Special case: eyelashes
    if EYES_NUM_CHOSEN:
        we_are_choosing_eyelashes = [leaf for leaf in leaves if 'eyelash' in leaf]
        if we_are_choosing_eyelashes:
            return pick_eyelashes_leaf(leaves)

    index = random.randint(0, len(leaves)-1)
    return leaves[index]


def pick_hand_06_leaf(leaves: List[str]):

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

    if DEBUG_LEVEL >= 1:
        sys.stderr.write(f"\tPicking hand 06 leaf from: {new_leaves}\n")

    index = random.randint(0, len(new_leaves)-1)
    return new_leaves[index]


def pick_eyelashes_leaf(leaves: List[str]):

    # Special case: eyelashes N only go with eye N
    if DEBUG_LEVEL >= 1:
        print_debug_message_once(f"\tFiltering eyelashes based on {EYES_NUM_CHOSEN}\n")

    new_leaves = [
        leaf for leaf in leaves
        if any([EYES_NUM_CHOSEN in leaf, "eyelashes_no" in leaf])
    ]

    if DEBUG_LEVEL >= 1:
        sys.stderr.write(f"\tPicking eyelashes leaf from: {new_leaves}\n")

    index = random.randint(0, len(new_leaves)-1)
    return new_leaves[index]


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


def pick_variant(variants: List[str]):

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
    we_are_choosing_hands = [v for v in variants if 'hand' in v]
    if we_are_choosing_hands:
        if MOUTH_06_HAS_BEEN_CHOSEN:
            variants = [v2 for v2 in variants if '6' in v2]
        # if mouth 06 hasn't been chosen, we can't choose hands 06
        else:
            variants = [v2 for v2 in variants if '6' not in v2]

    # Mouth 04 and 05 case: they can't go with Eyes 12
    if EYES_12_HAS_BEEN_CHOSEN:
        variants = [v for v in variants if not MOUTH_04_05_REGEX.match(v)]

    # Special case for ears 2: when we get to choose a head gadget,
    # we can only choose the 'head_gadget_plus_eyes' with 'no_gadget'
    if EARS_2_HAS_BEEN_CHOSEN:
        we_are_choosing_head_gadgets = [v for v in variants if 'head_gadget' in v]
        if we_are_choosing_head_gadgets:
            variants = [v for v in variants if 'no_gadget' in v]

    # Special case: some Mouths only go with some skins
    we_are_choosing_mouths = [v for v in variants if 'mouth_' in v]
    skin_has_mouth_filter = SKINS_TO_VALID_MOUTHS_MAP.get(SKIN_STREAM)

    if we_are_choosing_mouths and skin_has_mouth_filter:
        if DEBUG_LEVEL > 1:
            print_debug_message_once(f"\tFiltering mouths based on {SKIN_STREAM} skin\n")

        valid_mouths_numbers = [str(n) for n in skin_has_mouth_filter]
        filtered_variants = []
        for variant in variants:
            for valid_mouth_number in valid_mouths_numbers:
                if valid_mouth_number in variant:
                    filtered_variants.append(variant)
        if filtered_variants:
            variants = filtered_variants[:]

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


def choose_head_gadget(root_dir: str) -> str:
    variants = [
        d for d in os.listdir(root_dir)
        if os.path.isdir(os.path.join(root_dir, d))
    ]
    num_variants = len(variants)

    dir_index = random.randint(0, num_variants-1)
    random_dir = variants[dir_index]
    random_dir_path = os.path.join(root_dir, random_dir)

    files = [
        f for f
        in os.listdir(random_dir_path)
        if os.path.isfile(os.path.join(random_dir_path, f))
    ]

    file_index = random.randint(0, len(files)-1)
    return files[file_index].replace(".png", "")


def filter_based_on_head_gadget(leaves: List[str]) -> list:

    combinations = HEAD_GAGETS_COMBINATIONS.get(HEAD_GADGET_CHOSEN)
    if not combinations:
        return leaves

    leaves_to_keep = combinations['y']
    leaves_to_avoid = combinations['n']

    filtered_leaves = []

    # First pass: add variants to keep
    for _leaf in leaves:
        for to_keep in leaves_to_keep:
            if fnmatch.fnmatch(_leaf, to_keep):
                filtered_leaves.append(_leaf)

    # Second pass: remove variants to avoid
    returned_leaves = filtered_leaves[:]
    for _leaf_2 in filtered_leaves:
        for to_avoid in leaves_to_avoid:
            if fnmatch.fnmatch(_leaf_2, to_avoid):
                returned_leaves.pop(returned_leaves.index(_leaf_2))

    return returned_leaves


def traverse(tree: list, branch: list, parent_dir: str):

    # TODO: Find a way that doesn't rely on globals
    global SKIN_STREAM
    global EYES_12_HAS_BEEN_CHOSEN
    global MOUTH_06_HAS_BEEN_CHOSEN
    global MOUTH_06_HAS_GADGETS
    global EARS_2_HAS_BEEN_CHOSEN
    global HANDS_07_HAS_BEEN_CHOSEN
    global HEAD_GADGET_CHOSEN
    global EYES_NUM_CHOSEN

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

    # 0. If this is the first iteration, we need to chose a head gadget
    if os.path.basename(parent_dir) == "00_heads_assets":
        HEAD_GADGET_CHOSEN = choose_head_gadget(parent_dir)
        if DEBUG_LEVEL >= 1:
            sys.stderr.write(f"\t--> Head gadget chosen: {HEAD_GADGET_CHOSEN} <--\n")
        return

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

    # Special case: Head Gadgets
    filtered_leaves = potential_leaves
    if "09_head_gadget_plus_eyes" in parent_dir:
        filtered_leaves = filter_based_on_head_gadget(potential_leaves)

    if len(filtered_leaves) != len(potential_leaves):
        if DEBUG_LEVEL >= 2:
            sys.stderr.write(f"\tPotential leaves (after filtering): {filtered_leaves}\n")

    if not filtered_leaves:
        return

    final_leaf = pick_leaf(filtered_leaves)

    # Pick the skin / stream
    if branch[-1].lower() == SKINS_DIR_NAME:
        SKIN_STREAM = get_skin_stream(final_leaf)

        if DEBUG_LEVEL >= 1:
            sys.stderr.write(f"\t*** Chosen stream: '{SKIN_STREAM}'\n")

    # Are we in the special case of mouths 04/05 ?
    if final_leaf.startswith("eyes_12"):
        EYES_12_HAS_BEEN_CHOSEN = True
        if DEBUG_LEVEL >= 1:
            print_debug_message_once("\t--> Chosen eyes_12. Special behaviour will be activated.\n")

    # Are we in the special case of mouths/hands 06 ?
    if MOUTH_06_REGEX.match(final_leaf):
        MOUTH_06_HAS_BEEN_CHOSEN = True
        if DEBUG_LEVEL >= 1:
            print_debug_message_once("\t--> Chosen mouth_06. Special behaviour will be activated.\n")

    if MOUTH_06_HAS_GADGET_REGEX.match(final_leaf):
        MOUTH_06_HAS_GADGETS = True
        if DEBUG_LEVEL >= 1:
            print_debug_message_once("\t--> mouth_06 has gadgets. No gadgets should appear in 'hands'\n")

    # Special case for Ears 2
    if EARS_2_REGEX.match(final_leaf):
        EARS_2_HAS_BEEN_CHOSEN = True
        if DEBUG_LEVEL >= 1:
            print_debug_message_once("\t--> ears 2 has been chosen. 09_head_gadget should be 'no_gadget'\n")

    # Special case for Hands 7
    if "hand_7" in final_leaf:
        HANDS_07_HAS_BEEN_CHOSEN = True
        if DEBUG_LEVEL >= 1:
            print_debug_message_once("\t--> hands 7 has been chosen. We should have no mouth gadgets.\n")

    # Special case: eyelashes N only go with eye N
    if "eyes" in final_leaf and not EYES_NUM_CHOSEN:
        eyes_match = EYES_REGEX.match(final_leaf)
        EYES_NUM_CHOSEN = eyes_match.group()
        if DEBUG_LEVEL >= 1:
            print_debug_message_once(f"\t--> eyes num chosen: {EYES_NUM_CHOSEN}.\n")

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

    # FIXME: find a more elegant way
    # Special handling for hand 7, which is easier to do after creating the tree
    # since the traverse() algo is currently recursive
    new_tree = []
    if HANDS_07_HAS_BEEN_CHOSEN:
        for branch in tree:
            if "mouth" in branch and "_gadget" in branch:
                if DEBUG_LEVEL > 1:
                    sys.stderr.write(
                        (f"\tRemoving {branch} because we "
                         "have Hands 7, which are incompatible.\n"))
                continue

            new_tree.append(branch)

    if new_tree:
        final_tree = new_tree[:]
    else:
        final_tree = tree[:]

    recipe_stdout = []
    recipe_stdout.append(f"root_dir: {root_dir}")

    root_name = os.path.basename(root_dir)

    md5 = hashlib.md5()

    for branch in final_tree:
        branch_cleaned = branch[len(root_name)+1:]
        recipe_stdout.append(f"{branch_cleaned}")
        md5.update(branch_cleaned.encode("utf-8"))

    sys.stdout.write("\n".join(recipe_stdout))

    sys.stderr.write("\n")
    return md5.hexdigest()


# TODO: once a combination has been chosen, it can't be chosen again!
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

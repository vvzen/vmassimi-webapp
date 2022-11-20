#!/usr/bin/env python3
import os
import re
import sys
import string
import subprocess

UNDERSCORE_REPLACE_REGEX = re.compile(r'(_)+')
ARCHIVE_NAMING_REGEX = re.compile(r'([0-9]){3}')
NUM_PADDING = 2


def sanitize_name(file_name: str) -> str:
    if not isinstance(file_name, str):
        raise TypeError(("Please provide a string."
                         "Received: %s, %s") % (file_name, type(file_name)))

    # Skip the top-level archive directory
    if ARCHIVE_NAMING_REGEX.match(file_name):
        return file_name

    # Fix the entry point name
    if file_name == 'programm':
        return 'program'

    name, ext = os.path.splitext(file_name)

    name = name.replace(" ", "_")
    name = name.replace("-", "_")
    name = UNDERSCORE_REPLACE_REGEX.sub('_', name)

    # Replace special characters with something safer
    name = name.replace("+", "plus")
    name = name.replace("&", "_and_")

    # Add 0-padding if last token is a digit
    # Handle cases where the last token is already padded ('10, 11, ..')
    # TODO: this is messy, refactor into its own thing
    tokens = name.split('_')
    last_token = tokens[-1]
    should_add_underscore = False
    new_tokens = tokens[:]

    if len(last_token) > 1 and last_token[-1].isdigit():
        if len(last_token) != NUM_PADDING:
            new_tokens.insert(0, last_token[:-1])
            last_token = last_token[-1]

    if last_token.isdigit() and len(last_token) != NUM_PADDING:
        digit_as_str = str(int(last_token))
        new_token = digit_as_str.zfill(NUM_PADDING)
        new_tokens.pop()
        new_tokens.append(new_token)

    new_tokens = [
        t for t in new_tokens if t != "_"
    ]
    name = "_".join(new_tokens)

    letters = [
        a
        for a in name
        if any([[a in string.ascii_letters], [a in string.digits], [a in ("_", ".", )]])
    ]
    if ext:
        letters += [ext]

    return "".join(letters).lower()


def collect_operations(root_dir: str) -> list:

    rename_ops = []

    for root, dirs, files in os.walk(root_dir, topdown=False):
        base_name = os.path.basename(root)
        parent_dir = os.path.dirname(root)

        sanitized_name = sanitize_name(base_name)
        if base_name == sanitized_name:
            continue

        # Remove hidden and conf files
        for file in files:
            full_src_file_path = os.path.join(parent_dir, base_name, file)
            if file.startswith('.') or file.endswith('.ini'):
                rename_ops.append(f"rm -f '{full_src_file_path}'")
                continue

            # Sanitize file names
            file_sanitized_name = sanitize_name(file)
            if file_sanitized_name == file:
                continue

            full_dst_file_path = os.path.join(parent_dir, base_name, file_sanitized_name)
            rename_ops.append(f"mv '{full_src_file_path}' '{full_dst_file_path}'")

        # Sanitize directory names
        full_src_path = os.path.join(parent_dir, base_name)
        full_dst_path = os.path.join(parent_dir, sanitized_name)
        rename_ops.append(f"mv '{full_src_path}' '{full_dst_path}'")

    return rename_ops


def main():

    # Generate a sanitization script
    current_dir = os.path.dirname(__file__)
    target_dir = os.path.realpath(sys.argv[1])
    if not os.path.exists(target_dir):
        sys.stderr.write("%s does not exist on disk. Exiting..\n" % target_dir)
        sys.exit(1)

    rename_ops = collect_operations(target_dir)
    target_dir_name = os.path.basename(target_dir)
    target_dir_name = sanitize_name(target_dir_name)
    target_file = f"{current_dir}/operations_for_{target_dir_name}.sh"

    with open(target_file, "w") as f:
        f.write("#!/bin/bash\n")
        f.write("\n".join(rename_ops))

    os.chmod(target_file, mode=0o0755)

    # Run the sanitization
    p = subprocess.run([target_file], check=True)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import os
import sys
import string
import subprocess

UNDERSCORE_REPLACE_REGEX = re.compile(r'(_)+')


def sanitize_name(file_name: str) -> str:
    if not isinstance(file_name, str):
        raise TypeError(("Please provide a string."
                         "Received: %s, %s") % (file_name, type(file_name)))

    name = file_name.replace(" ", "_")
    name = name.replace("-", "_")
    name = name.replace("+", "plus")
    name = UNDERSCORE_REPLACE_REGEX.sub('_', name)

    letters = [
        a
        for a in name
        if any([[a in string.ascii_letters], [a in string.digits], [a in ("_", ".", )]])
    ]

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
    target_file = f"{current_dir}/operations_for_{target_dir_name}.sh"

    with open(target_file, "w") as f:
        f.write("#!/bin/bash\n")
        f.write("\n".join(rename_ops))

    os.chmod(target_file, mode=0o0755)

    # Run the sanitization
    p = subprocess.run([target_file], check=True)


if __name__ == "__main__":
    main()

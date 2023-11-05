#!/bin/python3

import os
import tomllib
import subprocess

from pathlib import Path

def main():
    root = os.getcwd()

    build_dir = root + "/build"
    scripts_dir = build_dir + "/scripts"

    create_mod_data(root, build_dir + "/core.mod")

    for path in [build_dir, scripts_dir]:
        if not os.path.isdir(path):
            os.mkdir(path)


    for name in os.listdir(root + "/scripts"):
        if not is_binary_target(root + "/scripts/" + name):
            continue

        compile_script(root + "/scripts/" + name)
        move_artifact(str(Path(root).parents[1]) + "/target/wasm32-unknown-unknown/debug/" + name + ".wasm", scripts_dir + "/" + name + ".wasm")

def is_binary_target(path):
    with open(path + "/Cargo.toml", "rb") as file:
        data = tomllib.load(file)

        try:
            crate_types = data["lib"]["crate-type"]
            for key in crate_types:
                if key == "cdylib":
                    return True

            return False
        except KeyError:
            return False

def compile_script(path):
    print("Building " + path)

    args = [
        "cargo",
        "+nightly",
        "build",
        "--target=wasm32-unknown-unknown",
    ]

    handle = subprocess.Popen(args, cwd=path)
    handle.wait()

def move_artifact(src, dst):
    print("mv ", src, dst)
    os.rename(src, dst)

def create_mod_data(root, dst):
    # Compile the json2dat tool
    workspace_root = str(Path(root).parents[1])

    handle = subprocess.Popen(["cargo", "build"], cwd=workspace_root + "/tools/json2dat")
    handle.wait()

    json2dat = workspace_root + "/target/debug/json2dat"

    handle = subprocess.Popen([json2dat, "--input", "mod.json", "--output", dst])
    code = handle.wait()

    if code != 0:
        print("Failed to create mod file")
        exit(1)

if __name__ == "__main__":
    main()

#!/bin/python3

import os
import sys
import subprocess

from pathlib import Path

PATH_BINARY = Path("target/release/builder")
PATH_SOURCE = Path("tools/builder")

def main():
    root = Path(__file__).parent.resolve()

    binary_path = root.joinpath(PATH_BINARY)
    source_path = root.joinpath(PATH_SOURCE)

    if need_rebuild(root):
        handle = subprocess.Popen(["cargo", "build", "--release"], cwd=source_path)
        status = handle.wait()
        if status != 0:
            print("failed to build builder")
            exit(1)
    
    handle = subprocess.Popen([binary_path] + sys.argv[1:], cwd=root)
    status = handle.wait()
    exit(status)

# Returns whether the binary should be rebuild before being executed.
def need_rebuild(root: Path) -> bool:
    binary_path = root.joinpath(PATH_BINARY)
    source_path = root.joinpath(PATH_SOURCE)

    # Build the binary if it does not exists.
    if not binary_path.exists():
        return True


    # Rebuild the binary if any source files have been updated since
    # the last build.
    mtime = binary_path.stat().st_mtime_ns
    for (dir, dirs, files) in os.walk(source_path):
        for file in files:
            file_path = Path(dir + "/" + file)
            if file_path.stat().st_mtime_ns > mtime:
                return True

    False

if __name__ == "__main__":
    main()

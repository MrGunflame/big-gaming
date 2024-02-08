#!/bin/python3

import os
import sys
import subprocess

from pathlib import Path

def main():
    cwd = os.getcwd()
    path = Path("target/release/builder")

    builder_path = Path("tools/builder")

    if not path.exists():
        handle = subprocess.Popen(["cargo", "build", "--release"], cwd=builder_path)
        status = handle.wait()
        if status != 0:
            print("failed to build builder")
            exit(1)
    
    handle = subprocess.Popen([path] + sys.argv[1:], cwd=cwd)
    status = handle.wait()
    exit(status)

if __name__ == "__main__":
    main()

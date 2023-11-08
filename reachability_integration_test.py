import os
import subprocess
import sys

TIMEOUT = sys.argv[1]
DIR = sys.argv[2]

if DIR != "fwd" and DIR != "bwd":
    print("Invalid direction. Allowed `fwd` or `bwd`.")
    exit(2)

os.system(f"cargo build --release --example test_reachability_{DIR}")
if os.path.isdir(f"./data/{DIR}-test-results"):
    print("Test results already exist. Won't overwrite existing data.")
    exit(2)

os.system(f"mkdir -p ./data/{DIR}-test-results")

files = list(os.listdir("./data/test-models"))
files = list(sorted(files))

for file in files:
    if not file.endswith(".sbml"):
        continue
    name = file.replace(".sbml", "")
    cmd_run = f"./target/release/examples/test_reachability_{DIR} ./data/test-models/{file} &> ./data/{DIR}-test-results/{name}.txt"
    code = os.system(f"timeout {TIMEOUT} {cmd_run}")
    if code == 31744 or code == 124:
        print(f"[PASS] No error discovered in `{file}` in less than {TIMEOUT}.")
    elif code != 0:
        print(f"[FAIL] Error ({code}) when testing `{file}`. See `./data/test-results/{name}.bwd_test.txt` for details.")
    else:
        print(f"[PASS] Completed `{file}`.")
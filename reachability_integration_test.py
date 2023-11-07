import os
import subprocess

TIMEOUT = "1m"

os.system("cargo build --release --example test_reachability_bwd")
if os.path.isdir("./data/test-results"):
    print("Test results already exist. Won't overwrite existing data.")
    exit(2)

os.system("mkdir -p ./data/test-results")

files = list(os.listdir("./data/test-models"))
files = list(sorted(files))

for file in files:
    if not file.endswith(".sbml"):
        continue
    name = file.replace(".sbml", "")
    cmd_run = f"./target/release/examples/test_reachability_bwd ./data/test-models/{file} &> ./data/test-results/{name}.bwd_test.txt"
    code = os.system(f"timeout {TIMEOUT} {cmd_run}")
    if code == 31744 or code == 124:
        print(f"[PASS] No error discovered in `{file}` in less than {TIMEOUT}.")
    elif code != 0:
        print(f"[FAIL] Error ({code}) when testing `{file}`. See `./data/test-results/{name}.bwd_test.txt` for details.")
    else:
        print(f"[PASS] Completed `{file}`.")
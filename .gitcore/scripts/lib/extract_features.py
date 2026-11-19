import json
import sys
from pathlib import Path

# Add script directory to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from load_lenient import load_lenient

if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
    print("Usage: python3 extract_features.py <path/to/features.json>", file=sys.stderr if len(sys.argv) < 2 else sys.stdout)
    if len(sys.argv) < 2:
        print("Error: Missing required argument: path to features.json", file=sys.stderr)
        sys.exit(1)
    sys.exit(0)

d = load_lenient(sys.argv[1])
for f in d.get("features", []):
    reqs = ",".join(f.get("req_ids", []))
    us = ",".join(f.get("user_stories", []))
    impl = f.get("implemented_in", "") or ""
    tests = ",".join(f.get("tests", [])) if isinstance(f.get("tests"), list) else str(f.get("tests", ""))
    print(f'{f["id"]}|{f.get("progress_pct",0)}|{reqs}|{us}|{impl}|{tests}')

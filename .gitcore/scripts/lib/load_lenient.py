import json
import sys
from pathlib import Path

# Add script directory to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent))


def load_lenient(path):
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        raw = f.read()
    out, in_str, escaped = [], False, False
    for c in raw:
        if in_str:
            if escaped:
                out.append(c)
                escaped = False
            elif c == "\\":
                out.append(c)
                escaped = True
            elif c == '"':
                in_str = False
                out.append(c)
            elif ord(c) < 0x20:
                out.append("\\u%04x" % ord(c))
            else:
                out.append(c)
        else:
            if c == '"':
                in_str = True
            out.append(c)
    return json.loads("".join(out))


if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print("Usage: python3 load_lenient.py <path/to/json_file>", file=sys.stderr if len(sys.argv) < 2 else sys.stdout)
        if len(sys.argv) < 2:
            print("Error: Missing required argument: path to JSON file", file=sys.stderr)
            sys.exit(1)
        sys.exit(0)

    d = load_lenient(sys.argv[1])
    print(json.dumps(d, indent=2, ensure_ascii=False))

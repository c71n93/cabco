"""Check Arguments/Returns contracts in compiler-produced Rustdoc JSON."""

import argparse
import json
import re
import sys
from pathlib import Path


def sections(docs):
    result = {}
    current = None
    fence = None
    for line in (docs or "").splitlines():
        marker = re.match(r"^ {0,3}(`{3,}|~{3,})", line)
        if marker:
            token = marker[1]
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
            continue
        if fence:
            continue
        heading = re.match(r"^#\s+(.+?)\s*#*\s*$", line)
        if heading:
            current = heading[1]
            result.setdefault(current, [])
        elif current:
            result[current].append(line)
    return {name: "\n".join(lines).strip() for name, lines in result.items()}


def check(document):
    if document["format_version"] != 54:
        raise ValueError("expected Rustdoc JSON format 54; use nightly-2025-07-30")
    errors = []
    for item in public_functions(document):
        function = item["inner"]["function"]
        contract = sections(item["docs"])
        parameters = [name for name, _ in function["sig"]["inputs"] if name != "self"]
        missing = []
        if parameters and not contract.get("Arguments"):
            missing.append("missing # Arguments")
        else:
            for name in parameters:
                entry = rf"(?m)^\s*[-*+]\s+`{re.escape(name)}`\s*(?::|[-–—])\s*\S"
                if not re.search(entry, contract.get("Arguments", "")):
                    missing.append(f"undocumented parameter `{name}` in # Arguments")
        output = function["sig"]["output"]
        if output is not None and output != {"tuple": []} and not contract.get("Returns"):
            missing.append("missing # Returns")
        span = item["span"]
        location = f'{span["filename"]}:{span["begin"][0]}' if span else "generated API"
        errors.extend(f'{location}: {item["name"]}: {message}' for message in missing)
    return errors


def public_functions(document):
    index = document["index"]
    pending = [document["root"]]
    visited = set()
    while pending:
        identifier = str(pending.pop())
        if identifier in visited or identifier not in index:
            continue
        visited.add(identifier)
        item = index[identifier]
        if item["crate_id"] != 0:
            continue
        kind, data = next(iter(item["inner"].items()))
        if kind == "function":
            yield item
        elif kind == "use":
            if data["id"] is not None:
                pending.append(data["id"])
        elif kind in ("module", "trait", "impl"):
            if kind == "impl" and data["trait"] is not None:
                continue  # Trait implementations inherit their declaration's contract.
            pending.extend(
                child for child in data["items"]
                if kind == "trait" or index[str(child)]["visibility"] == "public"
            )
        elif kind in ("struct", "enum", "union"):
            pending.extend(data["impls"])
            if kind == "struct" and isinstance(data["kind"], dict):
                fields = next(iter(data["kind"].values()))
                if isinstance(fields, dict):
                    fields = fields["fields"]
                pending.extend(field for field in fields if field is not None
                               and index[str(field)]["visibility"] == "public")
            elif kind == "enum":
                pending.extend(data["variants"])
            elif kind == "union":
                pending.extend(field for field in data["fields"]
                               if index[str(field)]["visibility"] == "public")
        elif kind == "variant":
            fields = data["kind"]
            if isinstance(fields, dict):
                fields = next(iter(fields.values()))
                if isinstance(fields, dict):
                    fields = fields["fields"]
                pending.extend(field for field in fields if field is not None)
        if kind not in ("module", "impl", "use"):
            pending.extend(type_references(data))


def type_references(value):
    if isinstance(value, dict):
        if "id" in value and value["id"] is not None:
            yield value["id"]
        for child in value.values():
            yield from type_references(child)
    elif isinstance(value, list):
        for child in value:
            yield from type_references(child)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("json", type=Path, help="Rustdoc JSON from the pinned nightly")
    args = parser.parse_args()
    try:
        errors = check(json.loads(args.json.read_text()))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Cannot check {args.json}: {error}", file=sys.stderr)
        return 1
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Rustdoc sections passed: {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

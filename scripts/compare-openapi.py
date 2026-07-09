#!/usr/bin/env python3
"""Compare OpenAPI specifications for endpoint compatibility."""

import json
import sys
from pathlib import Path


def main():
    if len(sys.argv) != 3:
        print("Usage: compare-openapi.py <reference-openapi.json> <local-openapi.json>")
        sys.exit(1)

    ref_path = Path(sys.argv[1])
    local_path = Path(sys.argv[2])

    # Load reference OpenAPI
    try:
        ref_data = json.loads(ref_path.read_text())
    except FileNotFoundError:
        print(f"Error: Reference file not found: {ref_path}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in reference file: {e}")
        sys.exit(1)

    # Load local OpenAPI
    try:
        local_data = json.loads(local_path.read_text())
    except FileNotFoundError:
        print(f"Error: Local file not found: {local_path}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in local file: {e}")
        sys.exit(1)

    ref_paths = ref_data.get("paths", {})
    local_paths = local_data.get("paths", {})

    http_methods = {"get", "put", "post", "delete", "options", "head", "patch", "trace"}

    has_differences = False

    # Check for missing paths
    missing_paths = set(ref_paths.keys()) - set(local_paths.keys())
    if missing_paths:
        has_differences = True
        for path in sorted(missing_paths):
            print(f"Missing path: {path}")

    # Check for extra paths
    extra_paths = set(local_paths.keys()) - set(ref_paths.keys())
    if extra_paths:
        has_differences = True
        for path in sorted(extra_paths):
            print(f"Extra path: {path}")

    # Check for method mismatches in common paths
    common_paths = set(ref_paths.keys()) & set(local_paths.keys())
    for path in sorted(common_paths):
        ref_methods = set(method.lower() for method in ref_paths[path].keys() if method.lower() in http_methods)
        local_methods = set(method.lower() for method in local_paths[path].keys() if method.lower() in http_methods)

        missing_methods = ref_methods - local_methods
        if missing_methods:
            has_differences = True
            for method in sorted(missing_methods):
                print(f"Missing method {method.upper()} for path: {path}")

        extra_methods = local_methods - ref_methods
        if extra_methods:
            has_differences = True
            for method in sorted(extra_methods):
                print(f"Extra method {method.upper()} for path: {path}")

    if has_differences:
        sys.exit(1)

    print(f"OpenAPI paths/methods match: {len(ref_paths)} paths")
    sys.exit(0)


if __name__ == "__main__":
    main()

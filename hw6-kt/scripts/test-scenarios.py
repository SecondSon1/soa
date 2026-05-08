#!/usr/bin/env python3.13
"""Run all test scenarios."""
import subprocess
import sys
import time
from pathlib import Path

scripts_dir = Path(__file__).parent

scenarios = sorted(scripts_dir.glob("scenario_*.py"))

failed = []
for scenario in scenarios:
    print()
    r = subprocess.run([sys.executable, str(scenario)], cwd=str(scripts_dir))
    if r.returncode != 0:
        failed.append(scenario.name)

print()
if failed:
    print(f"\033[0;31m>>> FAILED scenarios: {', '.join(failed)}\033[0m")
    sys.exit(1)
else:
    print(f"\033[0;32m>>> All {len(scenarios)} scenarios passed\033[0m")

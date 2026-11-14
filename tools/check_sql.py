#!/usr/bin/env python3
"""The generated SQL runs in real databases: every test diagram's scripts, as the snapshot tests keep
them (app/crates/core/tests/snapshots/), executed twice each (so that their DROP statements
are tried too):

  SQLite      in memory, with Python's own sqlite3
  PostgreSQL  in a throwaway local server (initdb, pg_ctl), where PostgreSQL is installed

    tools/check_sql.py
"""
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

SNAPSHOTS = Path(__file__).resolve().parent.parent / "app/crates/core/tests/snapshots"
PORT = "54331"


def scripts(dialect: str) -> list[tuple[str, str]]:
    """(snapshot, script) for every snapshot with a script in this dialect."""
    out = []
    for f in sorted(SNAPSHOTS.glob("*.snap")):
        m = re.search(rf"^== SQL \({dialect}\)\n(.*?)(?=^== |\Z)", f.read_text(), re.M | re.S)
        if m:
            out.append((f.stem.removeprefix("snapshots__"), m.group(1)))
    return out


def run_sqlite() -> list[str]:
    failed = []
    for name, sql in scripts("SQLite"):
        try:
            db = sqlite3.connect(":memory:")
            db.executescript(sql)
            db.executescript(sql)
            db.close()
        except sqlite3.Error as e:
            failed.append(f"{name}: {e}")
    print(f"sqlite: {len(scripts('SQLite'))} scripts, {len(failed)} failed")
    return failed


def run_postgresql() -> list[str]:
    if not (shutil.which("initdb") and shutil.which("pg_ctl") and shutil.which("psql")):
        print("postgresql: not installed, skipped")
        return []
    failed = []
    with tempfile.TemporaryDirectory() as tmp:
        db = Path(tmp) / "db"
        subprocess.run(["initdb", "-D", str(db), "-A", "trust", "-U", "postgres"], capture_output=True, check=True)
        subprocess.run(["pg_ctl", "-D", str(db), "-l", str(Path(tmp) / "log"), "-w", "-o",
                        f"-p {PORT} -k {tmp} -c listen_addresses=''", "start"], capture_output=True, check=True)
        psql = ["psql", "-h", tmp, "-p", PORT, "-U", "postgres", "-q"]
        try:
            for name, sql in scripts("PostgreSQL"):
                subprocess.run(psql + ["-c", "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"], capture_output=True)
                r = subprocess.run(psql + ["-v", "ON_ERROR_STOP=1"], input=sql + sql, capture_output=True, text=True)
                if r.returncode != 0:
                    failed.append(f"{name}: {r.stderr.strip()[:300]}")
        finally:
            subprocess.run(["pg_ctl", "-D", str(db), "-m", "fast", "stop"], capture_output=True)
    print(f"postgresql: {len(scripts('PostgreSQL'))} scripts, {len(failed)} failed")
    return failed


def main() -> int:
    failed = run_sqlite() + run_postgresql()
    for f in failed:
        print("FAIL", f)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

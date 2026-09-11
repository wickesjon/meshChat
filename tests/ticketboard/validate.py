"""Validate ticket metadata/DAG and regenerate the derived index and diagram.

No third-party dependencies. All writes stay inside this repository.
"""
from __future__ import annotations

import argparse
import heapq
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOARD = ROOT / "docs" / "ticketboard"
STATES = ("backlog", "inprogress", "inreview", "complete")
REQUIRED = (
    "Objective", "Dependencies", "Scope", "Implementation details",
    "Exit criteria", "Potential fallbacks", "Evidence", "Review and merge",
)


def inside(path: Path) -> Path:
    resolved = path.resolve()
    if not resolved.is_relative_to(ROOT):
        raise ValueError(f"Path escapes repository: {path}")
    return resolved


def parse_ticket(path: Path, state: str) -> dict:
    content = inside(path).read_text(encoding="utf-8")
    lines = content.splitlines()
    if not lines or lines[0] != "---":
        raise ValueError(f"{path.name}: missing front matter")
    try:
        end = lines.index("---", 1)
    except ValueError:
        raise ValueError(f"{path.name}: unclosed front matter") from None
    meta = {}
    for line in lines[1:end]:
        key, sep, raw = line.partition(":")
        if not sep or key in meta:
            raise ValueError(f"{path.name}: invalid/duplicate metadata key")
        try:
            meta[key] = json.loads(raw.strip())
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path.name}: invalid JSON metadata for {key}") from exc
    if set(meta) != {"id", "title", "depends_on", "kind", "branch"}:
        raise ValueError(f"{path.name}: unexpected or missing metadata fields")
    ticket_id = meta["id"]
    if not isinstance(ticket_id, str) or not re.fullmatch(r"MC-\d{3}", ticket_id):
        raise ValueError(f"{path.name}: invalid ID")
    if not path.name.startswith(ticket_id + "-"):
        raise ValueError(f"{path.name}: filename does not match ID")
    for key in ("title", "kind", "branch"):
        if not isinstance(meta[key], str) or not meta[key].strip():
            raise ValueError(f"{ticket_id}: empty/invalid {key}")
    if not re.fullmatch(r"ticket/" + ticket_id + r"-[a-z0-9]+(?:-[a-z0-9]+)*", meta["branch"]):
        raise ValueError(f"{ticket_id}: invalid ticket branch")
    deps = meta["depends_on"]
    if not isinstance(deps, list) or any(not isinstance(d, str) for d in deps):
        raise ValueError(f"{ticket_id}: depends_on must be a list of IDs")
    if len(deps) != len(set(deps)) or ticket_id in deps:
        raise ValueError(f"{ticket_id}: duplicate or self dependency")
    body = "\n".join(lines[end + 1:])
    for section in REQUIRED:
        match = re.search(r"^## " + re.escape(section) + r"\n(.*?)(?=^## |\Z)", body, re.M | re.S)
        if not match or not match.group(1).strip():
            raise ValueError(f"{ticket_id}: missing/empty {section}")
    if not re.search(r"^- \[[ x]\] ", body, re.M):
        raise ValueError(f"{ticket_id}: no checkable exit criteria")
    meta.update(path=path, state=state)
    return meta


def topological_order(tickets: dict) -> list[str]:
    counts = {key: len(t["depends_on"]) for key, t in tickets.items()}
    outgoing = {key: [] for key in tickets}
    for key, ticket in tickets.items():
        for dep in ticket["depends_on"]:
            if dep not in tickets:
                raise ValueError(f"{key}: missing dependency {dep}")
            outgoing[dep].append(key)
    ready = [key for key, count in counts.items() if not count]
    heapq.heapify(ready)
    result = []
    while ready:
        key = heapq.heappop(ready)
        result.append(key)
        for nxt in sorted(outgoing[key]):
            counts[nxt] -= 1
            if counts[nxt] == 0:
                heapq.heappush(ready, nxt)
    if len(result) != len(tickets):
        blocked = ", ".join(sorted(key for key, count in counts.items() if count))
        raise ValueError("Dependency cycle affects: " + blocked)
    return result


def derived_index(tickets: dict) -> str:
    rows = [
        "| Ticket | Task | State | Depends on |",
        "|---|---|---|---|",
    ]
    for key, ticket in sorted(tickets.items()):
        path = ticket["path"].relative_to(BOARD).as_posix()
        deps = ", ".join(ticket["depends_on"]) or "None"
        title = ticket["title"].replace("|", "\\|")
        rows.append(f"| [{key}]({path}) | {title} | {ticket['state']} | {deps} |")
    return "\n".join(rows)


def derived_graph(tickets: dict, order: list[str]) -> str:
    rows = ["```mermaid", "flowchart TD"]
    for key in order:
        label = key + ": " + tickets[key]["title"]
        label = label.replace("&", "and").replace('"', "'")
        rows.append(f'    {key.replace("-", "_")}["{label}"]')
    for key in order:
        for dep in tickets[key]["depends_on"]:
            rows.append(f'    {dep.replace("-", "_")} --> {key.replace("-", "_")}')
    rows.append("```")
    return "\n".join(rows)


def replace_block(content: str, tag: str, replacement: str) -> str:
    start, end = f"<!-- {tag}:START -->", f"<!-- {tag}:END -->"
    if content.count(start) != 1 or content.count(end) != 1:
        raise ValueError(f"Expected exactly one {tag} marker pair")
    before, tail = content.split(start)
    middle, after = tail.split(end)
    return before + start + "\n" + replacement + "\n" + end + after


def validate(write: bool = False) -> tuple[int, int]:
    tickets = {}
    for state in STATES:
        directory = inside(BOARD / state)
        if not directory.is_dir():
            raise ValueError(f"Missing workflow directory: {state}")
        for path in sorted(directory.glob("*.md")):
            ticket = parse_ticket(path, state)
            if ticket["id"] in tickets:
                raise ValueError(f"Duplicate ticket: {ticket['id']}")
            tickets[ticket["id"]] = ticket
    if not tickets:
        raise ValueError("No tickets found")
    order = topological_order(tickets)
    for key, ticket in tickets.items():
        if ticket["state"] in ("inprogress", "inreview", "complete"):
            unfinished = [dep for dep in ticket["depends_on"] if tickets[dep]["state"] != "complete"]
            if unfinished:
                raise ValueError(f"{key}: active/completed ticket has unfinished dependencies: {unfinished}")
    outputs = (
        (BOARD / "README.md", "INDEX", derived_index(tickets)),
        (BOARD / "implementation-plan.md", "DAG", derived_graph(tickets, order)),
    )
    stale = []
    prepared = []
    for path, tag, block in outputs:
        path = inside(path)
        old = path.read_text(encoding="utf-8")
        new = replace_block(old, tag, block)
        if old != new:
            stale.append(path.relative_to(ROOT).as_posix())
            prepared.append((path, new))
    if stale and not write:
        raise ValueError("Stale derived files; run with --write: " + ", ".join(stale))
    for path, new in prepared:
        path.write_text(new, encoding="utf-8", newline="\n")
    return len(tickets), sum(len(ticket["depends_on"]) for ticket in tickets.values())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="refresh derived index and DAG")
    args = parser.parse_args()
    try:
        count, edges = validate(args.write)
    except (ValueError, OSError) as exc:
        print(f"ERROR: {exc}")
        return 1
    print(f"OK: {count} tickets, {edges} dependencies; DAG and generated files valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

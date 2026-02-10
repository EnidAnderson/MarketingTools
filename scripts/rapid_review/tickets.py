#!/usr/bin/env python3
"""Manage Rapid Review Cell engineering tickets using append-only CSV logs."""

from __future__ import annotations

import argparse
import csv
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
LOG_DIR = ROOT / "planning" / "RAPID_REVIEW_CELL" / "logs"
CLAIM_REGISTER_PATH = LOG_DIR / "CLAIM_REGISTER.csv"
EVIDENCE_SUPPORT_PATH = LOG_DIR / "EVIDENCE_SUPPORT.csv"
DECISION_DISPOSITION_PATH = LOG_DIR / "DECISION_DISPOSITION.csv"
TICKET_QUEUE_PATH = LOG_DIR / "TICKET_QUEUE.csv"
TICKET_RESPONSES_PATH = LOG_DIR / "TICKET_RESPONSES.csv"

TICKET_QUEUE_FIELDS = [
    "row_id",
    "ts_utc",
    "ticket_id",
    "review_run_id",
    "artifact_id",
    "artifact_version",
    "claim_id",
    "priority",
    "status",
    "title",
    "requested_change",
    "acceptance_criteria",
    "owner_team",
    "opened_by",
    "supersedes_row_id",
]

TICKET_RESPONSE_FIELDS = [
    "row_id",
    "ts_utc",
    "ticket_id",
    "response_id",
    "responder_id",
    "response_type",
    "status_after",
    "change_ref",
    "verification_ref",
    "non_breaking_change",
    "notes",
    "supersedes_row_id",
]


@dataclass
class TicketSeed:
    review_run_id: str
    artifact_id: str
    artifact_version: str
    claim_id: str
    title: str
    priority: str
    requested_change: str
    acceptance_criteria: str


def now_utc_iso() -> str:
    return datetime.now(tz=timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def read_csv(path: Path) -> list[dict[str, str]]:
    if not path.exists():
        return []
    with path.open("r", encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def ensure_csv(path: Path, headers: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        return
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=headers)
        writer.writeheader()


def next_row_id(path: Path) -> int:
    rows = read_csv(path)
    if not rows:
        return 1
    return max(int(row["row_id"]) for row in rows if row.get("row_id", "").isdigit()) + 1


def append_row(path: Path, headers: list[str], row: dict[str, Any]) -> None:
    ensure_csv(path, headers)
    with path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=headers)
        writer.writerow({k: row.get(k, "") for k in headers})


def normalize_bool(value: str) -> str:
    lowered = value.strip().lower()
    if lowered in {"true", "t", "yes", "y", "1"}:
        return "true"
    if lowered in {"false", "f", "no", "n", "0"}:
        return "false"
    raise ValueError(f"Expected boolean-like value for non_breaking_change; got: {value}")


def claim_by_run(rows: list[dict[str, str]], review_run_id: str) -> dict[str, dict[str, str]]:
    out: dict[str, dict[str, str]] = {}
    for row in rows:
        if row.get("review_run_id") == review_run_id and row.get("claim_id"):
            out[row["claim_id"]] = row
    return out


def latest_status_by_ticket() -> dict[str, str]:
    status: dict[str, str] = {}
    for row in read_csv(TICKET_QUEUE_PATH):
        ticket_id = row.get("ticket_id")
        if ticket_id:
            status[ticket_id] = row.get("status", "open") or "open"
    for row in read_csv(TICKET_RESPONSES_PATH):
        ticket_id = row.get("ticket_id")
        if ticket_id and row.get("status_after"):
            status[ticket_id] = row["status_after"]
    return status


def next_ticket_id() -> str:
    rows = read_csv(TICKET_QUEUE_PATH)
    date_part = datetime.now(tz=timezone.utc).strftime("%Y%m%d")
    prefix = f"RRC-{date_part}-"
    max_idx = 0
    for row in rows:
        ticket_id = row.get("ticket_id", "")
        if ticket_id.startswith(prefix):
            try:
                max_idx = max(max_idx, int(ticket_id.split("-")[-1]))
            except ValueError:
                continue
    return f"{prefix}{max_idx + 1:03d}"


def create_ticket(args: argparse.Namespace) -> None:
    ticket_id = args.ticket_id or next_ticket_id()
    row = {
        "row_id": next_row_id(TICKET_QUEUE_PATH),
        "ts_utc": now_utc_iso(),
        "ticket_id": ticket_id,
        "review_run_id": args.review_run_id,
        "artifact_id": args.artifact_id,
        "artifact_version": args.artifact_version,
        "claim_id": args.claim_id,
        "priority": args.priority,
        "status": "open",
        "title": args.title,
        "requested_change": args.requested_change,
        "acceptance_criteria": args.acceptance_criteria,
        "owner_team": args.owner_team,
        "opened_by": args.opened_by,
        "supersedes_row_id": "",
    }
    append_row(TICKET_QUEUE_PATH, TICKET_QUEUE_FIELDS, row)
    print(f"Created ticket {ticket_id}")


def build_ticket_seeds_from_review(review_run_id: str) -> list[TicketSeed]:
    decisions = [
        row for row in read_csv(DECISION_DISPOSITION_PATH) if row.get("review_run_id") == review_run_id
    ]
    if not decisions:
        raise ValueError(f"No decision rows found for review_run_id={review_run_id}")

    latest_decision = decisions[-1]
    outcome = latest_decision.get("outcome", "")
    if outcome in {"approved_as_is"}:
        return []

    claims = claim_by_run(read_csv(CLAIM_REGISTER_PATH), review_run_id)
    evidence_rows = [row for row in read_csv(EVIDENCE_SUPPORT_PATH) if row.get("review_run_id") == review_run_id]

    seeds: list[TicketSeed] = []
    for row in evidence_rows:
        support_status = row.get("support_status", "")
        if support_status not in {"unsupported", "caveated", "aspirational"}:
            continue
        claim_id = row.get("claim_id", "")
        claim = claims.get(claim_id, {})
        normalized_claim = claim.get("normalized_claim", "").strip() or f"Claim {claim_id} requires correction"
        caveat_text = row.get("caveat_text", "").strip()
        requested_change = caveat_text or "Bind claim to concrete evidence or narrow the claim boundary."
        acceptance = (
            "Add supporting code/doc/test/artifact reference OR revise claim text to remove unsupported scope."
        )
        priority = "high" if support_status == "unsupported" else "medium"
        seeds.append(
            TicketSeed(
                review_run_id=review_run_id,
                artifact_id=row.get("artifact_id", ""),
                artifact_version=row.get("artifact_version", ""),
                claim_id=claim_id,
                title=f"{support_status.upper()}: {normalized_claim}",
                priority=priority,
                requested_change=requested_change,
                acceptance_criteria=acceptance,
            )
        )

    return seeds


def from_review(args: argparse.Namespace) -> None:
    seeds = build_ticket_seeds_from_review(args.review_run_id)
    if not seeds:
        print(f"No tickets required for review_run_id={args.review_run_id}")
        return

    created = 0
    for seed in seeds:
        ticket_id = next_ticket_id()
        row = {
            "row_id": next_row_id(TICKET_QUEUE_PATH),
            "ts_utc": now_utc_iso(),
            "ticket_id": ticket_id,
            "review_run_id": seed.review_run_id,
            "artifact_id": seed.artifact_id,
            "artifact_version": seed.artifact_version,
            "claim_id": seed.claim_id,
            "priority": seed.priority,
            "status": "open",
            "title": seed.title,
            "requested_change": seed.requested_change,
            "acceptance_criteria": seed.acceptance_criteria,
            "owner_team": args.owner_team,
            "opened_by": args.opened_by,
            "supersedes_row_id": "",
        }
        append_row(TICKET_QUEUE_PATH, TICKET_QUEUE_FIELDS, row)
        created += 1
        print(f"Created ticket {ticket_id} for claim {seed.claim_id}")

    print(f"Created {created} ticket(s) from review run {args.review_run_id}")


def respond(args: argparse.Namespace) -> None:
    queue = read_csv(TICKET_QUEUE_PATH)
    if not any(row.get("ticket_id") == args.ticket_id for row in queue):
        raise ValueError(f"Unknown ticket_id={args.ticket_id}")

    response_rows = read_csv(TICKET_RESPONSES_PATH)
    response_id = f"{args.ticket_id}-R{len([r for r in response_rows if r.get('ticket_id') == args.ticket_id]) + 1:02d}"
    row = {
        "row_id": next_row_id(TICKET_RESPONSES_PATH),
        "ts_utc": now_utc_iso(),
        "ticket_id": args.ticket_id,
        "response_id": response_id,
        "responder_id": args.responder_id,
        "response_type": args.response_type,
        "status_after": args.status_after,
        "change_ref": args.change_ref,
        "verification_ref": args.verification_ref,
        "non_breaking_change": normalize_bool(args.non_breaking_change),
        "notes": args.notes,
        "supersedes_row_id": "",
    }
    append_row(TICKET_RESPONSES_PATH, TICKET_RESPONSE_FIELDS, row)
    print(f"Added response {response_id} to {args.ticket_id}")


def list_tickets(args: argparse.Namespace) -> None:
    queue = read_csv(TICKET_QUEUE_PATH)
    if not queue:
        print("No tickets found.")
        return
    status_by_ticket = latest_status_by_ticket()
    rows = []
    for row in queue:
        ticket_id = row.get("ticket_id", "")
        current_status = status_by_ticket.get(ticket_id, row.get("status", "open"))
        if args.status and current_status != args.status:
            continue
        rows.append(
            (
                ticket_id,
                current_status,
                row.get("priority", ""),
                row.get("owner_team", ""),
                row.get("claim_id", ""),
                row.get("title", ""),
            )
        )

    if not rows:
        print("No tickets match the requested filters.")
        return

    for ticket_id, status, priority, owner_team, claim_id, title in rows:
        print(f"{ticket_id}\t{status}\t{priority}\t{owner_team}\t{claim_id}\t{title}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    parser_create = sub.add_parser("create", help="Create one engineering ticket.")
    parser_create.add_argument("--ticket-id", default="", help="Optional explicit ticket ID.")
    parser_create.add_argument("--review-run-id", required=True)
    parser_create.add_argument("--artifact-id", required=True)
    parser_create.add_argument("--artifact-version", required=True)
    parser_create.add_argument("--claim-id", required=True)
    parser_create.add_argument("--priority", choices=["low", "medium", "high"], default="medium")
    parser_create.add_argument("--title", required=True)
    parser_create.add_argument("--requested-change", required=True)
    parser_create.add_argument("--acceptance-criteria", required=True)
    parser_create.add_argument("--owner-team", required=True)
    parser_create.add_argument("--opened-by", default="review_cell")
    parser_create.set_defaults(func=create_ticket)

    parser_from_review = sub.add_parser(
        "from-review", help="Create tickets from unsupported/caveated claims in a review run."
    )
    parser_from_review.add_argument("--review-run-id", required=True)
    parser_from_review.add_argument("--owner-team", required=True)
    parser_from_review.add_argument("--opened-by", default="review_cell")
    parser_from_review.set_defaults(func=from_review)

    parser_respond = sub.add_parser("respond", help="Append an engineering response to a ticket.")
    parser_respond.add_argument("--ticket-id", required=True)
    parser_respond.add_argument("--responder-id", required=True)
    parser_respond.add_argument(
        "--response-type",
        choices=["analysis", "proposed_fix", "implemented", "verification", "blocked"],
        required=True,
    )
    parser_respond.add_argument(
        "--status-after",
        choices=["open", "in_progress", "awaiting_review", "resolved", "closed"],
        required=True,
    )
    parser_respond.add_argument(
        "--change-ref",
        required=True,
        help="Path(s), PR ref, or commit hash proving the change.",
    )
    parser_respond.add_argument(
        "--verification-ref",
        default="",
        help="Test report, screenshot, or check command proving expected behavior.",
    )
    parser_respond.add_argument(
        "--non-breaking-change",
        default="true",
        help="Whether the response preserves existing behavior (true/false).",
    )
    parser_respond.add_argument("--notes", required=True)
    parser_respond.set_defaults(func=respond)

    parser_list = sub.add_parser("list", help="List tickets.")
    parser_list.add_argument(
        "--status",
        choices=["open", "in_progress", "awaiting_review", "resolved", "closed"],
        default="",
    )
    parser_list.set_defaults(func=list_tickets)

    return parser


def main() -> None:
    ensure_csv(TICKET_QUEUE_PATH, TICKET_QUEUE_FIELDS)
    ensure_csv(TICKET_RESPONSES_PATH, TICKET_RESPONSE_FIELDS)

    parser = build_parser()
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()

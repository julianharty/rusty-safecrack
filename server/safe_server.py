#!/usr/bin/env python3
"""Reference safe-cracking server for CI and local testing.

This is an INDEPENDENT reimplementation of the observable contract of the
original "Safe Cracking Challenge" site (a stateful Flask app). It exists so the
Rust and TypeScript safecracker tools can be exercised end-to-end without the
original authors' (currently unpublished) source. It reproduces only the
request/response behaviour the tools rely on, derived from the captured
``rust/debug_*.html`` pages:

  * POST ``action=set_name``  &name=...      -> records a team name
  * POST ``action=select``    &param=&value= -> moves one dial (A/B/C/D)
  * POST ``action=add_attempt``              -> tests the current combination
  * GET  /                                   -> the current page

Session state (dial positions, attempts) is kept per browser session, keyed by a
``session`` cookie, exactly as the tools expect (they rotate sessions every 10
attempts).

The safe opens for the legitimate code (red, left, 0, alpha) and, by design, for
a two-setting interaction BUG: B=right AND D=gamma. That single injected bug is
caught by the pairwise sweep (combination red, right, 0, gamma), which is the
whole point of the exercise.

When the real Flask app is published it can replace this file wholesale — the
tools and the CI workflow only care about the URL and port.
"""

import argparse
import secrets
from flask import Flask, request, make_response

app = Flask(__name__)

# The legitimate combination the safe is documented to open with.
CORRECT = {"A": "red", "B": "left", "C": "0", "D": "alpha"}

# Allowed values per dial (used to validate `select` requests).
VALUES = {
    "A": ["red", "green", "blue"],
    "B": ["left", "middle", "right"],
    "C": ["0", "1", "2"],
    "D": ["alpha", "beta", "gamma"],
}

# In-memory session store: cookie token -> session state. Fine for a short-lived
# single-worker test server; not intended for production.
SESSIONS: dict[str, dict] = {}


def new_session() -> dict:
    """Fresh session with the dials at their documented baseline."""
    return {
        "name": None,
        "dials": dict(CORRECT),   # start at red / left / 0 / alpha
        "attempts": [],           # list of dicts: {combo, result}
        "last_open": False,       # drives the .display OPEN/CLOSED indicator
    }


def get_session() -> tuple[str, dict]:
    """Return (token, state) for the request, creating a session if needed."""
    token = request.cookies.get("session")
    if not token or token not in SESSIONS:
        token = secrets.token_hex(8)
        SESSIONS[token] = new_session()
    return token, SESSIONS[token]


def evaluate(dials: dict) -> tuple[bool, str]:
    """Decide whether the current dials open the safe.

    Returns (opened, message). The message text is what the tools grep for:
    the TypeScript tool looks for "wrong code" / "bug found" in the latest
    attempt row; both tools treat a non-"closed" .display as an opening.
    """
    if dials == CORRECT:
        return True, "Safe opened with the correct code"
    # Injected two-setting interaction bug: B=right AND D=gamma.
    if dials["B"] == "right" and dials["D"] == "gamma":
        return True, "Safe opened with a WRONG code - bug found!"
    return False, "Safe remained closed"


def render(state: dict) -> str:
    """Render the page the tools parse (classes: current-code, display, attempt)."""
    dials = state["dials"]
    display = "OPEN" if state["last_open"] else "CLOSED"

    current_code = "\n".join(
        f'<span>{p} = {dials[p]}</span>' for p in ("A", "B", "C", "D")
    )

    attempts_html = []
    for i, att in enumerate(state["attempts"], start=1):
        cls = "opened" if att["opened"] else "closed"
        values = "".join(f'<span>{p} = {att["combo"][p]}</span>' for p in ("A", "B", "C", "D"))
        attempts_html.append(
            f'<div class="attempt {cls}">'
            f'<h4>Attempt {i}</h4>'
            f'<div class="attempt-values">{values}</div>'
            f'<p class="small"><strong>{att["result"]}</strong></p>'
            f'</div>'
        )
    attempts_block = "\n".join(attempts_html) if attempts_html else "<p>No attempts yet.</p>"

    team = state["name"] or "Anonymous"
    used = len(state["attempts"])

    return f"""<!doctype html>
<html>
<head><meta charset="utf-8"><title>The Safe Cracking Challenge</title></head>
<body>
<div class="container">
<div class="card"><div class="status">Team: <strong>{team}</strong></div></div>
<div class="card">
<h2>Interactive Safe</h2>
<div class="safe"><div class="safe-grid">
<div class="display">
{display}
</div>
<div class="current-code">
{current_code}
</div>
</div></div>
<div class="status">Attempts used: {used}/10</div>
</div>
<div class="card">
<h2>Attempts</h2>
<div class="attempt-list">
{attempts_block}
</div>
</div>
</div>
</body>
</html>"""


@app.route("/", methods=["GET", "POST"])
def index():
    token, state = get_session()

    if request.method == "POST":
        action = request.form.get("action")
        if action == "set_name":
            state["name"] = request.form.get("name") or state["name"]
        elif action == "select":
            param = request.form.get("param")
            value = request.form.get("value")
            if param in VALUES and value in VALUES[param]:
                state["dials"][param] = value
                state["last_open"] = False  # moving a dial re-locks the display
        elif action == "add_attempt":
            opened, message = evaluate(state["dials"])
            state["last_open"] = opened
            state["attempts"].append(
                {"combo": dict(state["dials"]), "opened": opened, "result": message}
            )

    resp = make_response(render(state))
    resp.headers["Content-Type"] = "text/html; charset=utf-8"
    resp.set_cookie("session", token, httponly=True, samesite="Lax")
    return resp


def main() -> None:
    parser = argparse.ArgumentParser(description="Safe-cracking reference server")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=5004)
    args = parser.parse_args()
    # threaded=True so the tools' rapid sequential requests are served promptly.
    app.run(host=args.host, port=args.port, threaded=True)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Local full-stack smoke test. Uses SMTP stub; deletes only its own test accounts."""
import argparse
import json
import re
import secrets
import subprocess
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE = "http://127.0.0.1:8088"
COMPOSE = ["bash", str(ROOT / "scripts/compose.sh"), "app"]


def api(path, body=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    request = urllib.request.Request(BASE + path, data=None if body is None else json.dumps(body).encode(), headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=265) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        raw = error.read().decode()
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            payload = {"text": raw}
        return error.code, payload


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--include-gemini", action="store_true", help="one real message through the configured Gemini account")
    args = parser.parse_args()
    values = dict(line.split("=", 1) for line in (ROOT / "infra/.env").read_text().splitlines() if "=" in line and not line.startswith("#"))
    if values.get("SMTP_USER", "").strip() or values.get("SMTP_PASSWORD", "").strip():
        raise SystemExit("Этот тест требует пустых SMTP_USER/SMTP_PASSWORD: реальные письма не отправляются.")
    accounts = []
    tokens = []
    try:
        assert api("/api/v1/conversations")[0] == 401
        for _ in range(2):
            username = "integration_" + uuid.uuid4().hex[:14]
            status, result = api("/api/auth/auth/register", {"username": username, "email": username + "@gmail.com", "password": secrets.token_urlsafe(30)})
            assert status == 201, ("registration", status)
            owner = str(int(result["user_id"]))
            accounts.append(int(owner))
            pattern = r"body: Ваш код подтверждения: (\d+)\s+user_id=" + re.escape(owner) + r"(?:\s|$)"
            code = None
            for _ in range(40):
                logs = subprocess.run(COMPOSE + ["logs", "--no-color", "--no-log-prefix", "--tail=300", "notif"], cwd=ROOT, text=True, capture_output=True, check=True).stdout
                match = re.search(pattern, logs)
                if match:
                    code = match[1]
                    break
                time.sleep(0.2)
            assert code, "OTP was not delivered to the local SMTP stub"
            status, pair = api("/api/auth/auth/verify-email", {"user_id": owner, "code": code})
            assert status == 200, ("verify", status)
            tokens.append(pair["access_token"])
        first, second = tokens
        status, conversation = api("/api/v1/conversations", {}, first)
        assert status == 200
        path = "/api/v1/conversations/" + conversation["id"]
        assert api(path, token=first)[0] == 200
        assert api(path, token=second)[0] == 404
        status, history = api("/api/v1/conversations", token=second)
        assert status == 200 and not any(c["id"] == conversation["id"] for c in history["conversations"])
        fixture = json.loads((ROOT / "docs/rank-request.json").read_text())
        status, ranking = api("/api/v1/rank", fixture, first)
        assert status == 200 and ranking["slots"][0]["cards"][0]["score"] == 92.5
        print("Auth, email verification stub, JWT, conversations, owner isolation and ranking: OK")
        if args.include_gemini:
            status, reply = api(path + "/messages", {"request_id": str(uuid.uuid4()), "text": "Москва, центр. Японская кухня, чек до 1500 рублей. 1 ноября 2026 года обед с 13:00 до 15:00 по московскому времени. Программа не нужна, радиус 3000 метров."}, first)
            if status == 200:
                assert reply["type"] == "assistant"
                assert api(path, token=first)[1]["document"]["messages"]
                print("Configured Gemini returned an assistant response; history persisted: OK")
            elif status in (429, 503):
                print("Configured Gemini is externally blocked:", reply.get("text", "provider unavailable"))
            else:
                raise AssertionError(("unexpected Gemini response", status, reply.get("text")))
    finally:
        for token in tokens:
            try:
                api("/api/auth/auth/logout", {}, token)
            except Exception:
                pass
        if accounts:
            ids = ",".join(str(int(owner)) for owner in accounts)
            quoted_ids = ",".join("'" + str(int(owner)) + "'" for owner in accounts)
            sql = f"BEGIN; DELETE FROM gastro_conversations WHERE owner_id IN ({quoted_ids}); DELETE FROM users WHERE id IN ({ids}) AND username LIKE 'integration_%'; COMMIT;"
            subprocess.run(COMPOSE + ["exec", "-T", "postgres", "psql", "-U", "gastro", "-d", "users", "-v", "ON_ERROR_STOP=1"], cwd=ROOT, input=sql, text=True, capture_output=True, check=True)
            print("Temporary smoke-test accounts and conversations removed")


if __name__ == "__main__":
    main()

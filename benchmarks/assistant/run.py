"""Runs the assistant tasks through Tiles' Pi and model server, and reports how many pass."""

import argparse
import datetime
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import threading
import time
import urllib.request

from tasks import TASKS, Outcome, Task

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
TILES_DATA = pathlib.Path(os.environ.get("XDG_DATA_HOME", pathlib.Path.home() / ".local/share")) / "tiles"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--label", default="baseline", help="name for this run's results folder")
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--tasks", default="", help="comma-separated task ids or groups, all when empty")
    parser.add_argument("--modelfile", type=pathlib.Path, default=REPO / "modelfiles/gemma-4-12b-gguf")
    parser.add_argument("--pi", type=pathlib.Path, default=TILES_DATA / "pi/pi")
    parser.add_argument("--pi-agent", type=pathlib.Path, default=TILES_DATA / "data/pi/agent", help="where Tiles keeps Pi's models.json")
    parser.add_argument("--server", default="http://127.0.0.1:6969")
    parser.add_argument("--timeout", type=int, default=300, help="seconds for one chat")
    return parser.parse_args()


def system_prompt(modelfile: pathlib.Path) -> str:
    match = re.search(r'SYSTEM\s+"""(.*?)"""', modelfile.read_text(), re.DOTALL)
    return match.group(1).strip() if match else ""


def run_dates() -> dict[str, datetime.date]:
    today = datetime.date.today()
    return {
        "today": today,
        "tomorrow": today + datetime.timedelta(days=1),
        "in_4_days": today + datetime.timedelta(days=4),
        "in_9_days": today + datetime.timedelta(days=9),
    }


def copy_home(target: pathlib.Path, dates: dict[str, datetime.date]) -> None:
    shutil.copytree(HERE / "home", target)
    for path in target.rglob("*"):
        if path.is_file() and path.suffix in (".md", ".csv"):
            text = path.read_text()
            for name, day in dates.items():
                text = text.replace("{{" + name + "}}", day.strftime("%A %Y-%m-%d"))
            path.write_text(text)


def read_files(home: pathlib.Path) -> dict[str, str]:
    files = {}
    for path in sorted(home.rglob("*")):
        relative = path.relative_to(home).as_posix()
        if path.is_file() and not relative.startswith(".pi/") and path.name != ".keep":
            files[relative] = path.read_text(errors="replace")
    return files


class PiChat:
    """One Pi session in RPC mode, the way Tiles starts it."""

    def __init__(self, args: argparse.Namespace, agent_dir: pathlib.Path, home: pathlib.Path, prompt: str, log):
        env = dict(os.environ, PI_CODING_AGENT_DIR=str(agent_dir), PI_OFFLINE="true")
        self.process = subprocess.Popen(
            [str(args.pi), "--mode", "rpc", "--append-system-prompt", prompt, "--no-session"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            env=env,
            cwd=home,
        )
        self.log = log

    def ask(self, message: str, deadline: float) -> tuple[str, dict[str, int], int, bool]:
        self.process.stdin.write((json.dumps({"type": "prompt", "message": message}) + "\n").encode())
        self.process.stdin.flush()
        reply, calls, errors = "", {}, 0
        timer = threading.Timer(max(1, deadline - time.monotonic()), self.process.kill)
        timer.start()
        try:
            for line in self.process.stdout:
                self.log.write(line.decode(errors="replace"))
                event = json.loads(line)
                kind = event.get("type")
                if kind == "response" and not event.get("success", True):
                    break
                if kind == "message_end":
                    message_end = event.get("message", {})
                    content = message_end.get("content")
                    if message_end.get("role") == "assistant" and isinstance(content, list):
                        for item in content:
                            if item.get("type") == "toolCall":
                                calls[item.get("name", "?")] = calls.get(item.get("name", "?"), 0) + 1
                        text = "".join(i.get("text", "") for i in content if i.get("type") == "text")
                        reply = text if text.strip() else reply
                    if message_end.get("role") == "toolResult" and message_end.get("isError"):
                        errors += 1
                if kind == "agent_settled":
                    return reply, calls, errors, False
        finally:
            timer.cancel()
        return reply, calls, errors, True

    def close(self) -> None:
        self.process.kill()
        self.process.wait()


def run_task(task: Task, attempt: int, args, agent_dir, prompt, dates, out: pathlib.Path) -> dict:
    home = out / "homes" / f"{task.id}-{attempt}"
    copy_home(home, dates)
    before = read_files(home)
    replies, calls, errors, timed_out = [], {}, 0, False
    start = time.monotonic()
    with open(out / "logs" / f"{task.id}-{attempt}.jsonl", "w") as log:
        for messages in task.chats:
            chat = PiChat(args, agent_dir, home, prompt, log)
            deadline = time.monotonic() + args.timeout
            try:
                for message in messages:
                    reply, chat_calls, chat_errors, chat_timed_out = chat.ask(message, deadline)
                    errors += chat_errors
                    for name, count in chat_calls.items():
                        calls[name] = calls.get(name, 0) + count
                    if chat_timed_out:
                        timed_out = True
                        break
                replies.append(reply)
            finally:
                chat.close()
            if timed_out:
                break
    outcome = Outcome(replies=replies, before=before, after=read_files(home), dates=dates)
    passed, detail = task.check(outcome)
    return {
        "task": task.id,
        "group": task.group,
        "attempt": attempt,
        "passed": passed and not timed_out,
        "detail": "timed out" if timed_out else detail,
        "seconds": round(time.monotonic() - start, 1),
        "tool_calls": calls,
        "tool_errors": errors,
        "reply": outcome.reply,
    }


def print_report(results: list[dict]) -> None:
    print(f"\n{'task':20} {'passed':>7} {'avg s':>6} {'calls':>6}  failures")
    groups: dict[str, list[dict]] = {}
    for task in TASKS:
        runs = [r for r in results if r["task"] == task.id]
        if not runs:
            continue
        groups.setdefault(task.group, []).extend(runs)
        passed = sum(r["passed"] for r in runs)
        seconds = sum(r["seconds"] for r in runs) / len(runs)
        calls = sum(sum(r["tool_calls"].values()) for r in runs) / len(runs)
        failures = sorted({r["detail"] for r in runs if not r["passed"]})
        print(f"{task.id:20} {passed:>3}/{len(runs):<3} {seconds:>6.0f} {calls:>6.1f}  {'; '.join(failures)}")
    print()
    for group, runs in groups.items():
        print(f"{group:20} {sum(r['passed'] for r in runs):>3}/{len(runs)}")
    print(f"{'all':20} {sum(r['passed'] for r in results):>3}/{len(results)}")


def main() -> None:
    args = parse_args()
    try:
        urllib.request.urlopen(f"{args.server}/ping", timeout=5)
    except OSError:
        sys.exit(f"Tiles' model server isn't answering at {args.server}. Start it with `tiles server start`.")
    if not args.pi.exists():
        sys.exit(f"No Pi at {args.pi}. Point --pi at the one Tiles ships.")
    if not (args.pi_agent / "models.json").exists():
        sys.exit(f"No models.json in {args.pi_agent}. Run Tiles once so it writes Pi's config.")

    wanted = {name for name in args.tasks.split(",") if name}
    tasks = [t for t in TASKS if not wanted or t.id in wanted or t.group in wanted]
    prompt = system_prompt(args.modelfile)
    dates = run_dates()

    out = HERE / "results" / f"{datetime.datetime.now():%Y%m%d-%H%M}-{args.label}"
    agent_dir = out / "pi-agent"
    (out / "logs").mkdir(parents=True)
    agent_dir.mkdir()
    for name in ("models.json", "settings.json"):
        if (args.pi_agent / name).exists():
            shutil.copy(args.pi_agent / name, agent_dir / name)

    pi_version = subprocess.run([str(args.pi), "--version"], capture_output=True, text=True).stdout.strip()
    setup = {
        "label": args.label,
        "pi": str(args.pi),
        "pi_version": pi_version,
        "modelfile": str(args.modelfile),
        "system_prompt_sha256": hashlib.sha256(prompt.encode()).hexdigest()[:12],
        "models": json.loads((agent_dir / "models.json").read_text()),
        "server": args.server,
        "repeats": args.repeats,
    }
    print(f"{len(tasks)} tasks x {args.repeats}, results in {out.relative_to(REPO)}")

    results = []
    for attempt in range(1, args.repeats + 1):
        for task in tasks:
            result = run_task(task, attempt, args, agent_dir, prompt, dates, out)
            results.append(result)
            calls = sum(result["tool_calls"].values())
            print(
                f"[{attempt}/{args.repeats}] {task.id:20} {'PASS' if result['passed'] else 'FAIL'}  "
                f"{result['seconds']:>5.0f}s  {calls:>2} calls  {result['detail']}",
                flush=True,
            )
            (out / "summary.json").write_text(json.dumps({"setup": setup, "results": results}, indent=2))
    print_report(results)


if __name__ == "__main__":
    main()

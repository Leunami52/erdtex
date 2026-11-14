"""A small WebDriver client for the app window (tauri-driver + WebKitWebDriver), standard
library only.  The page offers `window.__erdtex` for what a user does through menus and dialogs."""
import base64
import json
import os
import socket
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path


def stop_leftovers(app: Path | None = None) -> None:
    """Ends what earlier test runs left: the WebDriver servers (test tools only) and the app under
    test, recognised by its path, so that an installed ERDTeX someone is using stays open."""
    for name in ("tauri-driver", "WebKitWebDriver"):
        subprocess.run(["pkill", "-x", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if app is not None:
        target = os.path.realpath(app)
        for pid in os.listdir("/proc"):
            if pid.isdigit():
                try:
                    if os.path.realpath(f"/proc/{pid}/exe") == target:
                        os.kill(int(pid), 9)
                except OSError:
                    pass
    time.sleep(0.3)


class Driver:
    def __init__(self, app: Path, env: dict | None = None, port: int = 4444):
        stop_leftovers(app)
        # ERDTEX_E2E_LOG=FILE: what the driver and the app print, for finding out why a session ended
        log = open(os.environ["ERDTEX_E2E_LOG"], "ab") if os.environ.get("ERDTEX_E2E_LOG") else subprocess.DEVNULL
        self.proc = subprocess.Popen([os.path.expanduser("~/.cargo/bin/tauri-driver"), "--port", str(port)],
                                     env={**os.environ, **(env or {}), "RUST_BACKTRACE": "1"}, stdout=log,
                                     stderr=log)
        self.base = f"http://127.0.0.1:{port}"
        for _ in range(100):
            try:
                socket.create_connection(("127.0.0.1", port), timeout=0.2).close()
                break
            except OSError:
                time.sleep(0.1)
        caps = {"capabilities": {"alwaysMatch": {"tauri:options": {"application": str(app)}}}}
        self.sid = None
        try:
            for attempt in range(20):        # the drivers take a moment to start
                try:
                    self.sid = self.request("POST", "/session", caps)["sessionId"]
                    break
                except (OSError, RuntimeError):
                    if attempt == 19:
                        raise
                    time.sleep(0.3)
            self.wait("return !!window.__erdtex", 20)
        except BaseException:
            self.quit()
            raise

    def request(self, method: str, path: str, body=None):
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(self.base + path, data=data, method=method,
                                     headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.loads(r.read() or b"{}").get("value")
        except urllib.error.HTTPError as e:
            raise RuntimeError(f"{method} {path}: {e.read().decode(errors='replace')[:500]}") from None

    def session(self, method: str, path: str, body=None):
        return self.request(method, f"/session/{self.sid}{path}", body)

    def js(self, script: str, *args):
        return self.session("POST", "/execute/sync", {"script": script, "args": list(args)})

    def call(self, expr: str, *args):
        """Awaits a JavaScript expression (it sees the arguments as `args`)."""
        script = ("const done = arguments[arguments.length - 1]; const args = [...arguments].slice(0, -1);"
                  f"Promise.resolve().then(() => ({expr})).then(v => done({{ok: v === undefined ? null : v}}),"
                  " e => done({err: String(e)}));")
        r = self.session("POST", "/execute/async", {"script": script, "args": list(args)})
        if "err" in r:
            raise RuntimeError(r["err"])
        return r["ok"]

    def wait(self, script: str, timeout: float = 10):
        end = time.time() + timeout
        while time.time() < end:
            try:
                if self.js(script):
                    return
            except RuntimeError:
                pass
            time.sleep(0.05)
        raise TimeoutError(script)

    def frames(self, n: int = 2):
        """Waits until the page has painted."""
        self.call("new Promise(r => { let k = %d; const f = () => --k ? requestAnimationFrame(f) : r(); requestAnimationFrame(f); })" % n)

    KEYS = {"Control": "\ue009", "Shift": "\ue008", "Alt": "\ue00a", "Escape": "\ue00c", "Enter": "\ue007",
            "Space": " ", "Delete": "\ue017", "Left": "\ue012", "Up": "\ue013", "Right": "\ue014",
            "Down": "\ue015", "PgUp": "\ue00e", "PgDown": "\ue00f", "F1": "\ue031", "F5": "\ue035", "F6": "\ue036", "Backspace": "\ue003", "Tab": "\ue004",
            "Home": "\ue011", "End": "\ue010"}

    def keys(self, *combo: str):
        """Presses keys together, e.g. keys("Control", "z")."""
        codes = [self.KEYS.get(k, k) for k in combo]
        actions = [{"type": "keyDown", "value": c} for c in codes] + [{"type": "keyUp", "value": c} for c in reversed(codes)]
        self.session("POST", "/actions", {"actions": [{"type": "key", "id": "kb", "actions": actions}]})
        self.session("DELETE", "/actions")

    def type(self, text: str):
        """Types text one key after the other ("\n" is Enter)."""
        acts = []
        for ch in text:
            c = self.KEYS["Enter"] if ch == "\n" else ch
            acts += [{"type": "keyDown", "value": c}, {"type": "keyUp", "value": c}]
        self.session("POST", "/actions", {"actions": [{"type": "key", "id": "kb", "actions": acts}]})
        self.session("DELETE", "/actions")

    def pointer(self, steps: list, hold: tuple = ()):
        """Real mouse input: steps are ("move", x, y) in viewport pixels, ("down", button),
        ("up", button) or ("pause", ms); `hold`: keys held down meanwhile (Shift, Alt ...)."""
        acts = []
        for s in steps:
            if s[0] == "move":
                acts.append({"type": "pointerMove", "duration": 0, "x": int(round(s[1])), "y": int(round(s[2])),
                             "origin": "viewport"})
            elif s[0] in ("down", "up"):
                acts.append({"type": "pointerDown" if s[0] == "down" else "pointerUp", "button": s[1]})
            else:
                acts.append({"type": "pause", "duration": s[1]})
        codes = [self.KEYS.get(k, k) for k in hold]
        pause = {"type": "pause", "duration": 0}
        pointer = [pause] * len(codes) + acts + [pause] * len(codes)
        sources = [{"type": "pointer", "id": "mouse", "parameters": {"pointerType": "mouse"}, "actions": pointer}]
        if codes:
            keys = ([{"type": "keyDown", "value": c} for c in codes] + [pause] * len(acts)
                    + [{"type": "keyUp", "value": c} for c in reversed(codes)])
            sources.append({"type": "key", "id": "kb", "actions": keys})
        self.session("POST", "/actions", {"actions": sources})
        self.session("DELETE", "/actions")

    def rect(self, css: str) -> dict:
        return self.js("const r = document.querySelector(arguments[0]).getBoundingClientRect();"
                       "return {x: r.left, y: r.top, w: r.width, h: r.height};", css)

    def screenshot(self, path: Path, css: str | None = None):
        if css:
            el = self.session("POST", "/element", {"using": "css selector", "value": css})
            eid = next(iter(el.values()))
            png = self.session("GET", f"/element/{eid}/screenshot")
        else:
            png = self.session("GET", "/screenshot")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(base64.b64decode(png))

    def quit(self):
        try:
            if self.sid:
                self.request("DELETE", f"/session/{self.sid}")
        except Exception:  # noqa: BLE001 - the app may have ended already
            pass
        self.proc.terminate()
        self.proc.wait(timeout=10)

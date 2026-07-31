import os
from typing import Any, Dict
from urllib.parse import urlparse

from playwright.async_api import async_playwright


class BrowserTool:
    def __init__(self):
        self._policy: str | None = None
        self._domains: list[str] = []

    def _load_policy(self) -> None:
        """Load network policy from environment variables."""
        if self._policy is not None:
            return
        self._policy = os.environ.get("AGENTICBOX_NETWORK_POLICY", "offline")
        domains_str = os.environ.get("AGENTICBOX_NETWORK_DOMAINS", "")
        self._domains = [d.strip() for d in domains_str.split(",") if d.strip()]

    def _check_url(self, url: str) -> None:
        """Check if a URL is allowed by the network policy. Raises ValueError if blocked."""
        self._load_policy()
        if self._policy == "full":
            return
        if self._policy == "offline":
            raise ValueError(f"Network is offline — cannot navigate to {url}")
        if self._policy == "localhost":
            if "localhost" not in url and "127.0.0.1" not in url:
                raise ValueError(f"Only localhost allowed, but URL is: {url}")
            return
        if self._policy == "allowlist":
            parsed = urlparse(url)
            hostname = parsed.hostname or ""
            allowed = any(
                hostname == domain or hostname.endswith("." + domain)
                for domain in self._domains
            )
            if not allowed:
                raise ValueError(
                    f"URL '{url}' is not in the allowed domains: {', '.join(self._domains)}"
                )
            return
        raise ValueError(f"Network is offline — cannot navigate to {url}")

    def definition(self) -> Dict[str, Any]:
        return {
            "name": "browser",
            "description": "Automate browser actions (network policy enforced)",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["navigate", "screenshot", "click", "type", "extract"],
                    },
                    "url": {"type": "string"},
                    "selector": {"type": "string"},
                    "text": {"type": "string"},
                },
                "required": ["action"],
            },
        }

    async def invoke(self, args: Dict[str, Any]) -> Dict[str, Any]:
        action = args["action"]
        url = args.get("url", "")

        # Check network policy before any action that involves a URL
        if url:
            self._check_url(url)

        async with async_playwright() as p:
            browser = await p.chromium.launch(headless=True)
            ctx = await browser.new_context()
            page = await ctx.new_page()
            try:
                if action == "navigate":
                    await page.goto(args["url"])
                    return {"title": await page.title(), "url": page.url}
                elif action == "screenshot":
                    if args.get("url"):
                        await page.goto(args["url"])
                    data = await page.screenshot()
                    return {"screenshot": data.hex()}
                elif action == "click":
                    await page.click(args["selector"])
                    return {"status": "clicked"}
                elif action == "type":
                    await page.fill(args["selector"], args["text"])
                    return {"status": "typed"}
                elif action == "extract":
                    text = await page.evaluate("() => document.body.innerText")
                    return {"text": text}
            finally:
                await browser.close()

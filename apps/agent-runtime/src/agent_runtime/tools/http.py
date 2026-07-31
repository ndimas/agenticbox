import os
from typing import Any, Dict
from urllib.parse import urlparse

import httpx


class HTTPTool:
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
            raise ValueError(f"Network is offline — cannot make request to {url}")
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
            "name": "http",
            "description": "Make HTTP requests (network policy enforced)",
            "parameters": {
                "type": "object",
                "properties": {
                    "method": {"type": "string", "enum": ["GET", "POST"]},
                    "url": {"type": "string"},
                    "headers": {"type": "object"},
                    "body": {"type": "string"},
                },
                "required": ["method", "url"],
            },
        }

    async def invoke(self, args: Dict[str, Any]) -> Dict[str, Any]:
        method = args["method"]
        url = args["url"]
        headers = args.get("headers", {})
        body = args.get("body")

        # Check network policy before making the request
        self._check_url(url)

        async with httpx.AsyncClient(timeout=30) as client:
            if method == "GET":
                r = await client.get(url, headers=headers)
            else:
                r = await client.post(url, headers=headers, content=body)
            return {
                "status_code": r.status_code,
                "headers": dict(r.headers),
                "body": r.text,
            }

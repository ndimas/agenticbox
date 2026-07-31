"""Tests for the browser tool's network policy enforcement."""

import os
import unittest


class TestBrowserNetworkPolicy(unittest.TestCase):
    """Test the _check_url method of BrowserTool without launching Playwright."""

    def setUp(self):
        # Import here to avoid playwright dependency in unit tests
        from agent_runtime.tools.browser import BrowserTool

        self.tool = BrowserTool()
        # Force reload of policy
        self.tool._policy = None

    def set_policy(self, policy: str, domains: str = ""):
        os.environ["AGENTICBOX_NETWORK_POLICY"] = policy
        os.environ["AGENTICBOX_NETWORK_DOMAINS"] = domains
        self.tool._policy = None
        self.tool._domains = []

    # ── Full policy ───────────────────────────────────────

    def test_full_allows_any_url(self):
        self.set_policy("full")
        self.tool._check_url("https://evil.attacker.com/exfil")
        self.tool._check_url("https://api.openai.com/v1/models")

    # ── Offline policy ────────────────────────────────────

    def test_offline_blocks_all(self):
        self.set_policy("offline")
        with self.assertRaises(ValueError) as ctx:
            self.tool._check_url("https://api.openai.com/v1/models")
        assert "offline" in str(ctx.exception).lower()
        with self.assertRaises(ValueError):
            self.tool._check_url("http://localhost:3000")

    # ── Localhost policy ──────────────────────────────────

    def test_localhost_allows_localhost_urls(self):
        self.set_policy("localhost")
        self.tool._check_url("http://localhost:3000")
        self.tool._check_url("http://127.0.0.1:8080")

    def test_localhost_blocks_external(self):
        self.set_policy("localhost")
        with self.assertRaises(ValueError) as ctx:
            self.tool._check_url("https://api.openai.com/v1/models")
        assert "localhost" in str(ctx.exception).lower()

    # ── Allowlist policy ──────────────────────────────────

    def test_allowlist_allows_listed_domain(self):
        self.set_policy("allowlist", "api.openai.com, github.com")
        self.tool._check_url("https://api.openai.com/v1/models")
        self.tool._check_url("https://github.com/repos/morpheus-sh/agenticbox")

    def test_allowlist_allows_subdomain(self):
        """Subdomain of allowed domain should be allowed (e.g., api.github.com when github.com is allowed)."""
        self.set_policy("allowlist", "github.com")
        self.tool._check_url("https://api.github.com/user")
        self.tool._check_url("https://docs.github.com/repos")

    def test_allowlist_blocks_unlisted_domain(self):
        self.set_policy("allowlist", "api.openai.com")
        with self.assertRaises(ValueError) as ctx:
            self.tool._check_url("https://evil.attacker.com/exfil")
        assert "not in the allowed domains" in str(ctx.exception)

    def test_allowlist_blocks_when_empty(self):
        self.set_policy("allowlist", "")
        with self.assertRaises(ValueError):
            self.tool._check_url("https://api.openai.com/v1/models")

    # ── Subdomain spoofing protection ──────────────────────

    def test_allowlist_blocks_subdomain_spoof(self):
        """Subdomain prefix must not bypass allowlist (e.g. evil-api.openai.com when api.openai.com is allowed)."""
        self.set_policy("allowlist", "api.openai.com")
        with self.assertRaises(ValueError) as ctx:
            self.tool._check_url("https://evil-api.openai.com/exfil")
        assert "not in the allowed domains" in str(ctx.exception)

    def test_allowlist_blocks_suffix_spoof(self):
        """Domain suffix must not bypass allowlist (e.g. notopenai.com when openai.com is allowed)."""
        self.set_policy("allowlist", "openai.com")
        with self.assertRaises(ValueError) as ctx:
            self.tool._check_url("https://notopenai.com/exfil")
        assert "not in the allowed domains" in str(ctx.exception)

    def test_allowlist_allows_exact_match(self):
        """Exact domain match should still work."""
        self.set_policy("allowlist", "api.openai.com")
        self.tool._check_url("https://api.openai.com/v1/models")

    # ── Default policy ────────────────────────────────────

    def test_default_policy_is_offline(self):
        # No env vars set
        self.tool._policy = None
        self.tool._domains = []
        if "AGENTICBOX_NETWORK_POLICY" in os.environ:
            del os.environ["AGENTICBOX_NETWORK_POLICY"]
        with self.assertRaises(ValueError):
            self.tool._check_url("https://api.openai.com")

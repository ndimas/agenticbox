"""Tests for the HTTP tool's network policy enforcement."""

import os
import unittest


class TestHTTPNetworkPolicy(unittest.TestCase):
    """Test the _check_url method of HTTPTool without making real HTTP requests."""

    def setUp(self):
        from agent_runtime.tools.http import HTTPTool

        self.tool = HTTPTool()
        self.tool._policy = None

    def set_policy(self, policy: str, domains: str = ""):
        os.environ["AGENTICBOX_NETWORK_POLICY"] = policy
        os.environ["AGENTICBOX_NETWORK_DOMAINS"] = domains
        self.tool._policy = None
        self.tool._domains = []

    def test_full_allows_any_url(self):
        self.set_policy("full")
        self.tool._check_url("https://evil.attacker.com/exfil")
        self.tool._check_url("https://api.openai.com/v1/models")

    def test_offline_blocks_all(self):
        self.set_policy("offline")
        with self.assertRaises(ValueError):
            self.tool._check_url("https://api.openai.com/v1/models")

    def test_allowlist_allows_listed_domain(self):
        self.set_policy("allowlist", "api.openai.com, github.com")
        self.tool._check_url("https://api.openai.com/v1/models")
        self.tool._check_url("https://github.com/repos/morpheus-sh/agenticbox")

    def test_allowlist_blocks_unlisted_domain(self):
        self.set_policy("allowlist", "api.openai.com")
        with self.assertRaises(ValueError):
            self.tool._check_url("https://evil.attacker.com/exfil")

    # ── Subdomain spoofing protection ──────────────────────

    def test_allowlist_blocks_subdomain_spoof(self):
        """Subdomain prefix must not bypass allowlist."""
        self.set_policy("allowlist", "api.openai.com")
        with self.assertRaises(ValueError):
            self.tool._check_url("https://evil-api.openai.com/exfil")

    def test_allowlist_blocks_suffix_spoof(self):
        """Domain suffix must not bypass allowlist."""
        self.set_policy("allowlist", "openai.com")
        with self.assertRaises(ValueError):
            self.tool._check_url("https://notopenai.com/exfil")

    def test_allowlist_allows_exact_match(self):
        """Exact domain match should still work."""
        self.set_policy("allowlist", "api.openai.com")
        self.tool._check_url("https://api.openai.com/v1/models")

    def test_allowlist_allows_subdomain(self):
        """Subdomain of allowed domain should be allowed."""
        self.set_policy("allowlist", "github.com")
        self.tool._check_url("https://docs.github.com/repos")

from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]


def load(relative: str):
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


class MarketplaceTests(unittest.TestCase):
    def test_catalogs_reference_the_same_plugin(self) -> None:
        codex = load(".agents/plugins/marketplace.json")
        claude = load(".claude-plugin/marketplace.json")

        self.assertEqual(codex["name"], "smartpowers")
        self.assertEqual(claude["name"], "smartpowers")
        self.assertEqual(codex["plugins"][0]["name"], "smartpowers")
        self.assertEqual(claude["plugins"][0]["name"], "smartpowers")
        self.assertEqual(
            codex["plugins"][0]["source"]["path"], "./plugins/smartpowers"
        )
        self.assertEqual(
            claude["plugins"][0]["source"], "./plugins/smartpowers"
        )

    def test_plugin_manifests_stay_in_sync(self) -> None:
        codex = load("plugins/smartpowers/.codex-plugin/plugin.json")
        claude = load("plugins/smartpowers/.claude-plugin/plugin.json")

        self.assertEqual(codex["name"], claude["name"])
        self.assertEqual(codex["version"], claude["version"])
        self.assertEqual(codex["description"], claude["description"])
        self.assertTrue((ROOT / "plugins/smartpowers/skills").is_dir())

    def test_board_crate_version_matches_plugin(self) -> None:
        plugin = load("plugins/smartpowers/.claude-plugin/plugin.json")
        board = ROOT / "plugins/smartpowers/skills/smartpowers/board"
        expected = f'version = "{plugin["version"]}"'

        manifest = (board / "Cargo.toml").read_text(encoding="utf-8")
        self.assertIn(expected, manifest.splitlines())
        lock = (board / "Cargo.lock").read_text(encoding="utf-8")
        self.assertIn(f'name = "smartpowers-board"\n{expected}', lock)


if __name__ == "__main__":
    unittest.main()

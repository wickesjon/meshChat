"""Focused contract tests for the ticketboard validator (stdlib only)."""
import unittest
from pathlib import Path
from unittest.mock import patch

import validate


class DependencyTests(unittest.TestCase):
    def test_valid_graph_orders_prerequisites_first(self):
        tickets = {
            "MC-001": {"depends_on": []},
            "MC-002": {"depends_on": ["MC-001"]},
            "MC-003": {"depends_on": ["MC-001", "MC-002"]},
        }
        self.assertEqual(validate.topological_order(tickets), list(tickets))

    def test_missing_dependency_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "missing dependency"):
            validate.topological_order({"MC-001": {"depends_on": ["MC-999"]}})

    def test_cycle_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "cycle"):
            validate.topological_order({
                "MC-001": {"depends_on": ["MC-002"]},
                "MC-002": {"depends_on": ["MC-001"]},
            })

    def test_graph_arrows_run_from_prerequisite_to_dependent(self):
        tickets = {
            "MC-001": {"depends_on": [], "title": "First"},
            "MC-002": {"depends_on": ["MC-001"], "title": "Second"},
        }
        graph = validate.derived_graph(tickets, validate.topological_order(tickets))
        self.assertIn("MC_001 --> MC_002", graph)
        self.assertNotIn("MC_002 --> MC_001", graph)


class MetadataTests(unittest.TestCase):
    def setUp(self):
        self.path = validate.BOARD / "backlog" / "MC-001-example.md"
        self.content = (
            '---\nid: "MC-001"\ntitle: "Example"\ndepends_on: []\n'
            'kind: "docs"\nbranch: "ticket/MC-001-example"\n---\n\n'
            + "\n\n".join("## " + section + "\n\n- [ ] Evidence." for section in validate.REQUIRED)
        )

    def parse(self, content):
        with patch.object(Path, "read_text", return_value=content):
            return validate.parse_ticket(self.path, "backlog")

    def test_valid_metadata(self):
        self.assertEqual(self.parse(self.content)["id"], "MC-001")

    def test_self_duplicate_and_malformed_dependencies_rejected(self):
        for deps in ('["MC-001"]', '["MC-002", "MC-002"]', '"MC-002"'):
            with self.subTest(deps=deps), self.assertRaises(ValueError):
                self.parse(self.content.replace("depends_on: []", "depends_on: " + deps))

    def test_main_branch_rejected(self):
        with self.assertRaisesRegex(ValueError, "branch"):
            self.parse(self.content.replace("ticket/MC-001-example", "main"))

    def test_empty_required_section_rejected(self):
        with self.assertRaisesRegex(ValueError, "Potential fallbacks"):
            self.parse(self.content.replace("## Potential fallbacks\n\n- [ ] Evidence.", "## Potential fallbacks\n"))

    def test_duplicate_metadata_key_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.parse(self.content.replace('kind: "docs"', 'kind: "docs"\nkind: "gate"'))

    def test_repository_escape_rejected(self):
        with self.assertRaisesRegex(ValueError, "escapes repository"):
            validate.inside(validate.ROOT / ".." / "outside.md")


class GenerationTests(unittest.TestCase):
    def test_replacement_preserves_surrounding_content_and_is_idempotent(self):
        original = "before\n<!-- DAG:START -->\nold\n<!-- DAG:END -->\nafter"
        changed = validate.replace_block(original, "DAG", "new")
        self.assertEqual(changed, "before\n<!-- DAG:START -->\nnew\n<!-- DAG:END -->\nafter")
        self.assertEqual(validate.replace_block(changed, "DAG", "new"), changed)

    def test_missing_or_duplicate_markers_rejected(self):
        for content in ("none", "<!-- DAG:START -->\n<!-- DAG:START -->\n<!-- DAG:END -->"):
            with self.subTest(content=content), self.assertRaises(ValueError):
                validate.replace_block(content, "DAG", "new")


if __name__ == "__main__":
    unittest.main()

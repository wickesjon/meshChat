"""Keep the original native palettes identical across Android and iOS."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[3]


class Palettes(unittest.TestCase):
    def test_native_palette_parity(self):
        def palettes(path):
            result = {}
            for line in path.read_text().splitlines():
                if 'ThemeTokens(' not in line or '0x' not in line:
                    continue
                name = re.search(r'"([a-z]+)"', line).group(1)
                result[name] = [int(value, 16) & 0xffffff for value in re.findall(r'0x([A-Fa-f0-9]+)', line)]
            return result
        android = palettes(ROOT/'src/android/ui/src/main/kotlin/org/meshchat/ui/ThemeTokens.kt')
        ios = palettes(ROOT/'src/ios/UI/ThemeTokens.swift')
        self.assertEqual(len(android), 5)
        self.assertEqual(android, ios)


if __name__ == '__main__':
    unittest.main()

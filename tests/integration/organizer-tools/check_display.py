"""Local Tk layout and matrix checks; requires a display, uses no real secrets."""
import importlib.util
from pathlib import Path
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('console', ROOT / 'src/organizer-tools/console.py')
console = importlib.util.module_from_spec(spec)
spec.loader.exec_module(console)
console.tk, console.filedialog, console.messagebox, console.ttk = tk, filedialog, messagebox, ttk
window = tk.Tk()
window.withdraw()
app = console.Console(window)
window.update_idletasks()
assert app.unlock.cget('show') != ''
assert window.winfo_reqwidth() <= 900
assert window.winfo_reqheight() <= 900
fixtures = dict(line.split('\t', 1) for line in (ROOT / '.work/mc041/fixtures.tsv').read_text().splitlines())
canvas = tk.Canvas(window)
rows = fixtures['matrix'].split('/')
console.draw_matrix(canvas, rows)
assert len(canvas.find_all()) == sum(row.count('1') for row in rows)
scale = max(2, min(6, 520 // (len(rows) + 8)))
assert all(min(canvas.coords(item)) >= 4 * scale for item in canvas.find_all())
canvas.delete('all')
assert not canvas.find_all()
window.destroy()
print('MC-041 Tk layout, masked unlock entry, quiet zone and QR clearing PASS; no screenshot-protection claim')

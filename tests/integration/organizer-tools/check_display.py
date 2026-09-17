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
# Exercise the real modal's timeout/close callback with an accelerated Tk clock.
# Keep it withdrawn so no private fixture appears on the developer's display.
original_dialog = tk.Toplevel
scheduled, cleared = [], []
class HiddenDialog(original_dialog):
    def __init__(self, parent):
        super().__init__(parent)
        self.withdraw()
    def grab_set(self):
        pass  # An invisible test window cannot take a native modal grab.
    def after(self, milliseconds, callback=None, *args):
        scheduled.append(milliseconds)
        return super().after(1, callback, *args)
    def destroy(self):
        qr = [child for child in self.winfo_children() if isinstance(child, tk.Canvas)]
        assert qr and all(not child.find_all() for child in qr)
        cleared.append(True)
        super().destroy()

console.tk.Toplevel = HiddenDialog
console.messagebox.askokcancel = lambda *args, **kwargs: True
console.backend = lambda request: ({'name': 'Synthetic event', 'expiry': 202000}
                                  if request['operation'] == 'check-staff' else
                                  {'matrix': rows, 'label': 'Test staff'})
folder = ROOT / '.work/mc041/console-check'
folder.mkdir(exist_ok=True)
dummy = folder / 'display-test-placeholder'
dummy.write_text('synthetic vault placeholder')
app.vault.insert(0, str(dummy)); app.unlock.insert(0, '11' * 32)
app.label.insert(0, 'Test staff')
app.before.insert(0, '2026-09-17 00:00'); app.after.insert(0, '2026-09-17 01:00')
app.issue(False)
assert scheduled == [60000] and cleared == [True]
assert app.unlock.get() == ''
console.tk.Toplevel = original_dialog
window.destroy()
print('MC-041 Tk layout, masked/cleared unlock, QR quiet zone and timed one-time display clearing PASS; accelerated UI clock, no screenshot-protection claim')

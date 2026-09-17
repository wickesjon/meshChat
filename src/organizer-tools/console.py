"""Local offline organizer console. No network, clipboard or private file export."""
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def backend(request):
    suffix = '.exe' if sys.platform == 'win32' else ''
    executable = ROOT / 'target/release' / ('meshchat-organizer' + suffix)
    if not executable.is_file():
        raise ValueError('Build the release organizer tool first; see docs/organizer/offline-console.md.')
    result = subprocess.run([str(executable), '--private-pipe'], input=json.dumps(request).encode(),
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30,
                            creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == 'win32' else 0)
    if result.returncode:
        raise ValueError('Operation refused. Check the dates, label, vault and unlock code.')
    return json.loads(result.stdout)


def timestamp(value):
    date = datetime.strptime(value, '%Y-%m-%d %H:%M').replace(tzinfo=timezone.utc)
    seconds = int(date.timestamp())
    if not 0 < seconds <= 0xffffffff:
        raise ValueError('Date is outside the supported range.')
    return seconds


def write_new(path, value):
    # Never overwrite another event or follow an existing output symlink.
    with Path(path).open('x', encoding='utf-8') as target:
        target.write(value + '\n')


def draw_matrix(canvas, rows):
    width = len(rows)
    if not 21 <= width <= 177 or any(len(row) != width or set(row) - {'0', '1'} for row in rows):
        raise ValueError('Invalid QR matrix.')
    scale = max(2, min(6, 520 // (width + 8)))
    canvas.configure(width=(width + 8) * scale, height=(width + 8) * scale, bg='white')
    for y, row in enumerate(rows):
        for x, pixel in enumerate(row):
            if pixel == '1':
                left, top = (x + 4) * scale, (y + 4) * scale
                canvas.create_rectangle(left, top, left + scale, top + scale, fill='black', outline='')


class Console:
    def __init__(self, window):
        self.window = window
        window.title('meshChat · Offline organizer')
        window.report_callback_exception = lambda *_: messagebox.showerror(
            'Operation stopped', 'The operation failed. No private diagnostic data was logged.', parent=window)
        box = ttk.Frame(window, padding=24)
        box.pack(fill='both', expand=True)
        ttk.Label(box, text='Offline organizer', font=('', 20, 'bold')).pack(anchor='w')
        ttk.Label(box, text='Create an event root, then issue a separate code for each staff phone.\n'
                  'Keep this computer offline. Never provision a relay beacon.', wraplength=580).pack(anchor='w', pady=(8, 18))
        tabs = ttk.Notebook(box)
        tabs.pack(fill='both', expand=True)
        create, issue = ttk.Frame(tabs, padding=18), ttk.Frame(tabs, padding=18)
        tabs.add(create, text='Create event'); tabs.add(issue, text='Provision staff')
        self.name = self.field(create, 'Event name (up to 32 UTF-8 bytes)')
        self.expiry = self.field(create, 'Root expiry · UTC · YYYY-MM-DD HH:MM')
        ttk.Label(create, text='The root is saved encrypted. A random unlock code is displayed once.\n'
                  'Record it securely and separately; a lost code cannot be recovered.', wraplength=540).pack(anchor='w', pady=12)
        ttk.Button(create, text='Check event inputs', command=lambda: self.guard(self.check_root)).pack(anchor='w')
        ttk.Button(create, text='Create encrypted root…', command=lambda: self.guard(self.create)).pack(anchor='w', pady=10)
        self.vault = self.field(issue, 'Encrypted root file')
        ttk.Button(issue, text='Choose root file…', command=self.choose).pack(anchor='w')
        self.unlock = self.field(issue, 'Unlock code (64 hexadecimal characters)', secret=True)
        ttk.Button(issue, text='Export public adoption link…', command=lambda: self.guard(self.export_public)).pack(anchor='w', pady=8)
        self.label = self.field(issue, 'Staff label (up to 16 UTF-8 bytes)')
        self.before = self.field(issue, 'Shift starts · UTC · YYYY-MM-DD HH:MM')
        self.after = self.field(issue, 'Shift ends · UTC · YYYY-MM-DD HH:MM')
        ttk.Button(issue, text='Check without issuing', command=lambda: self.guard(lambda: self.issue(True))).pack(anchor='w', pady=10)
        ttk.Button(issue, text='Show one-time staff code…', command=lambda: self.guard(lambda: self.issue(False))).pack(anchor='w')
        ttk.Label(box, text='Nothing is sent online. The private staff QR is never saved or copied.\n'
                  'Protect this screen from cameras, screenshots and screen sharing.', wraplength=580).pack(anchor='w', pady=(18, 0))

    @staticmethod
    def field(parent, label, secret=False):
        ttk.Label(parent, text=label).pack(anchor='w', pady=(10, 3))
        entry = ttk.Entry(parent, width=62, show='•' if secret else '')
        entry.pack(fill='x')
        return entry

    def guard(self, action):
        try:
            action()
        except (ValueError, OSError, subprocess.SubprocessError):
            messagebox.showerror('Operation refused', 'Check the inputs and dates, the selected root, and its unlock code.\n'
                                 'Existing files are never overwritten. See the offline console guide.', parent=self.window)

    def root_request(self, operation):
        return {'operation': operation, 'name': self.name.get(), 'expiry': timestamp(self.expiry.get())}

    def check_root(self):
        backend(self.root_request('check-root'))
        messagebox.showinfo('Inputs valid', 'No key was created. Check the event name and UTC expiry before creating.', parent=self.window)

    def create(self):
        request = self.root_request('create')
        backend({**request, 'operation': 'check-root'})
        if not messagebox.askokcancel('Confirm event', f"Create {request['name']}?\nRoot expires {self.expiry.get()} UTC.", parent=self.window):
            return
        path = filedialog.asksaveasfilename(parent=self.window, title='Save encrypted root', defaultextension='.mcvault')
        if not path:
            return
        result = backend(request)
        write_new(path, result.pop('vault'))
        self.vault.delete(0, 'end'); self.vault.insert(0, path)
        dialog = tk.Toplevel(self.window); dialog.title('Record the root unlock code once')
        dialog.transient(self.window); dialog.grab_set()
        ttk.Label(dialog, text='Record this unlock code securely, separately from the encrypted root.\n'
                  'It will not be displayed again. It is never part of a QR code.', padding=18).pack()
        secret = ttk.Label(dialog, text=result.pop('unlock'), font=('Courier', 12), padding=18)
        secret.pack()
        event = result['event']
        def close():
            secret.configure(text=''); dialog.destroy()
        def export():
            destination = filedialog.asksaveasfilename(parent=dialog, title='Save PUBLIC adoption link', defaultextension='.txt')
            if destination:
                self.guard(lambda: write_new(destination, event))
        ttk.Button(dialog, text='Save public adoption link…', command=export).pack(pady=8)
        ttk.Button(dialog, text='I recorded the unlock code · close', command=close).pack(pady=18)
        dialog.protocol('WM_DELETE_WINDOW', close)
        self.window.wait_window(dialog)
        result.clear()

    def choose(self):
        path = filedialog.askopenfilename(parent=self.window, title='Choose encrypted event root')
        if path:
            self.vault.delete(0, 'end'); self.vault.insert(0, path)

    def export_public(self):
        unlock = self.unlock.get(); self.unlock.delete(0, 'end')
        with Path(self.vault.get()).open(encoding='utf-8') as source:
            vault = source.read(4097).strip()
        request = {'operation': 'inspect', 'vault': vault, 'unlock': unlock}
        unlock = None
        try:
            result = backend(request)
        finally:
            request.clear()
        destination = filedialog.asksaveasfilename(parent=self.window, title='Save PUBLIC adoption link', defaultextension='.txt')
        if destination:
            write_new(destination, result['event'])

    def issue(self, dry_run):
        unlock = self.unlock.get(); self.unlock.delete(0, 'end')
        with Path(self.vault.get()).open(encoding='utf-8') as source:
            vault = source.read(4097).strip()
        request = {'operation': 'check-staff', 'vault': vault, 'unlock': unlock,
                   'label': self.label.get(), 'before': timestamp(self.before.get()), 'after': timestamp(self.after.get())}
        unlock = None
        try:
            preview = backend(request)
            if dry_run:
                messagebox.showinfo('Valid staff request', f"Event: {preview['name']}\nNo staff key was generated. Re-enter the unlock code to issue.", parent=self.window)
                return
            if not messagebox.askokcancel('Confirm staff provisioning',
                    f"Event: {preview['name']}\nStaff: {request['label']}\n"
                    f"Valid {self.before.get()} through {self.after.get()} UTC.\n"
                    'Show a private code for 60 seconds? Scan only on the intended staff phone.', parent=self.window):
                return
            request['operation'] = 'issue'
            result = backend(request)
        finally:
            request.clear()
        dialog = tk.Toplevel(self.window); dialog.title('Private staff provisioning · one time')
        dialog.transient(self.window); dialog.grab_set()
        ttk.Label(dialog, text=f"{preview['name']} · {result['label']}\nScan on the intended staff phone only. Closes in 60 seconds.", padding=14).pack()
        canvas = tk.Canvas(dialog, highlightthickness=0)
        canvas.pack(padx=18, pady=8)
        draw_matrix(canvas, result.pop('matrix'))
        result.clear()
        def close():
            if dialog.winfo_exists():
                canvas.delete('all'); dialog.destroy()
        timer = dialog.after(60000, close)
        ttk.Button(dialog, text='Scanned · erase code now', command=close).pack(pady=14)
        dialog.protocol('WM_DELETE_WINDOW', close)
        self.window.wait_window(dialog)
        self.window.after_cancel(timer)


if __name__ == '__main__':
    import tkinter as tk
    from tkinter import filedialog, messagebox, ttk
    app = tk.Tk()
    Console(app)
    app.mainloop()

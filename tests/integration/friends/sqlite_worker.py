"""Test-only unencrypted SQLite bridge for Rust policy integration.

The actual production SQL statements and transactions run here. A fake
cipher_version identifies this explicitly as a double, not encryption evidence.
Only synthetic fixtures are sent over the local pipes or written under .work.
"""
import sqlite3
import sys

db = sqlite3.connect(sys.argv[1], isolation_level=None)
failure = None
for line in sys.stdin:
    try:
        mode, limit, sql, *values = line.strip().split()
        sql = bytes.fromhex(sql).decode()
        if mode == 'f':
            failure = sql if sql != '-' else None
            print('ok 0', flush=True)
            continue
        if failure and sql.startswith(failure):
            raise ValueError('injected')
        values = [int(x[1:]) if x[0] == 'i' else bytes.fromhex(x[1:]) if x[0] == 'b' else bytes.fromhex(x[1:]).decode() for x in values]
        if sql == 'PRAGMA cipher_version':
            rows = [('unencrypted-test-double',)]
        else:
            cursor = db.execute(sql, values)
            if mode == 'x':
                if cursor.description is not None:
                    raise ValueError('row-returning execute')
                rows = []
            else:
                rows = cursor.fetchmany(int(limit) + 1)
        print('ok ' + str(len(rows)))
        for row in rows:
            print(' '.join('i' + str(x) if isinstance(x, int) else 'b' + x.hex() if isinstance(x, bytes) else 't' + x.encode().hex() for x in row))
        sys.stdout.flush()
    except Exception:
        print('error', flush=True)

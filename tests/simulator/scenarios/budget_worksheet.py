"""MC-007 arithmetic/scheduling witness, not a mesh implementation test."""
from collections import deque
import json
from math import ceil
from pathlib import Path

MANIFEST = json.loads(Path(__file__).with_name('MC-007-acceptance.json').read_text(encoding='utf-8'))
SYNC = MANIFEST['sync']
CAPACITY = MANIFEST['common']['capacity_bytes_each_direction']
QUANTA = (8, 16, 8, 4, 8)


def frame_sizes(size, transport=False):
    whole = 5 if transport else 4
    overhead = 16 if transport else 18
    if size + whole <= CAPACITY:
        return [size + whole]
    chunk = CAPACITY - overhead
    return [min(chunk, size - i) + overhead for i in range(0, size, chunk)]


class Direction:
    def __init__(self, name, logical_size):
        self.name = name
        self.logical_size = logical_size
        self.queues = [deque() for _ in QUANTA]
        self.sync = deque()
        self.announce = deque()
        self.prefer_sync = True
        self.deficits = [0] * len(QUANTA)
        self.index = 0
        self.enter = True
        self.active = None
        self.trace = []
        self.received = 0
        self.complete_at = None
        self.next_data = 0
        self.peer = None
        self.crypto = [(-1, 'proof_reservation', 8)]

    def enqueue(self, kind, size, transport=False, item=None):
        obj = dict(kind=kind, sizes=frame_sizes(size, transport), item=item)
        if kind in ('data', 'page', 'terminal'):
            self.sync.append(obj)
        elif kind == 'announce':
            self.announce.append(obj)
        else:
            self.queues[0 if kind == 'request' else 1].append(obj)

    def queue(self):
        if self.index != 4:
            return self.queues[self.index]
        if self.sync and self.announce:
            return self.sync if self.prefer_sync else self.announce
        return self.sync or self.announce

    def choose(self):
        for _ in range(2 * len(QUANTA)):
            q = self.queue()
            if self.enter:
                self.deficits[self.index] = min(32, self.deficits[self.index] + QUANTA[self.index])
                self.enter = False
            if q and len(q[0]['sizes']) <= self.deficits[self.index]:
                obj = q.popleft()
                self.deficits[self.index] -= len(obj['sizes'])
                if self.index == 4:
                    self.prefer_sync = obj['kind'] == 'announce'
                return obj
            if not q:
                self.deficits[self.index] = 0
            self.index = (self.index + 1) % len(QUANTA)
            self.enter = True
        return None

    def send_tick(self, time):
        if self.active is None:
            self.active = self.choose()
        if self.active is None:
            return None
        obj = self.active
        size = obj['sizes'].pop(0)
        self.trace.append((time, obj['kind'], size))
        if obj['sizes']:
            return None
        self.active = None
        return obj

    def receive(self, obj, time):
        kind = obj['kind']
        if kind == 'data':
            self.crypto.append((time, kind, SYNC['max_work_units_per_item']))
        elif kind == 'announce':
            self.crypto.append((time, kind, 1))
        if kind == 'request':
            self.enqueue('data', self.logical_size + 11, True, self.next_data)
        elif kind == 'data':
            assert obj['item'] == self.received
            self.received += 1
        elif kind == 'page':
            self.enqueue('request', 546)
        elif kind == 'terminal':
            assert self.received == SYNC['items_each_direction']
            self.complete_at = time

    def sent(self, obj):
        if obj['kind'] == 'data':
            self.next_data += 1
            if self.next_data == SYNC['items_each_direction']:
                self.enqueue('terminal', 11, True)
            elif self.next_data % SYNC['page_items'] == 0:
                self.enqueue('page', 11, True)
            else:
                self.enqueue('data', self.logical_size + 11, True, self.next_data)


def check_bucket(trace, capacity, refill, cost):
    tokens, previous = capacity, trace[0][0]
    minimum = tokens
    for time, kind, size in trace:
        tokens = min(capacity, tokens + (time - previous) * refill)
        charge = cost(kind, size)
        assert tokens >= charge, (time, kind, tokens, charge)
        tokens -= charge
        minimum = min(minimum, tokens)
        previous = time
    return minimum


def witness(logical_size):
    a, b = Direction('A', logical_size), Direction('B', logical_size)
    a.peer, b.peer = b, a
    for d in (a, b):
        d.enqueue('request', 546)
    for time in range(SYNC['duration_seconds']):
        for d in (a, b):
            if time in SYNC['announce_times_seconds']:
                d.enqueue('announce', SYNC['announce_size_bytes'])
                d.crypto.append((time, 'sign_announce', 1))
            if time in SYNC['reaction_times_seconds']:
                d.enqueue('reaction', SYNC['reaction_size_bytes'])
        completed = [(d, d.send_tick(time)) for d in (a, b)]
        for d, obj in completed:
            if obj is not None:
                d.sent(obj)
                d.peer.receive(obj, time + 0.02)
    fragments = ceil((logical_size + 11) / (CAPACITY - 16))
    expected_frames = 8 * fragments + 40
    expected_bytes = 8 * (logical_size + 11 + fragments * 16) + 2 * 16 + 2 * (546 + 5 * 18) + 4 * (421 + 4 * 18) + 12 * 40
    if logical_size == 1024:
        assert expected_frames == 104 and expected_bytes == 13060
    for d in (a, b):
        assert d.received == SYNC['items_each_direction'] and d.complete_at <= SYNC['pass']['completion_seconds_max']
        assert len(d.trace) == expected_frames, len(d.trace)
        assert sum(row[2] for row in d.trace) == expected_bytes
        setup = [(-9, 'hello', 59)] + [(t, 'proof_reservation', 146) for t in range(-8, 0)]
        assert len(setup) == SYNC['setup_reserved_frames_each_direction']
        assert sum(row[2] for row in setup) == SYNC['setup_reserved_bytes_each_direction']
        all_frames = setup + d.trace
        check_bucket(all_frames, 15, 1, lambda k, n: 1)
        check_bucket(all_frames, 24 * 1024, 2 * 1024, lambda k, n: n)
        check_bucket(all_frames, 60, 8, lambda k, n: 1)
        check_bucket(all_frames, 64 * 1024, 8 * 1024, lambda k, n: n)
        check_bucket(sorted(d.crypto), 20, 20, lambda k, n: n)
        check_bucket(sorted(d.crypto), 40, 40, lambda k, n: n)
        check_bucket(d.trace, 120, 2, lambda k, n: int(k in ('data', 'page', 'terminal')))
        assert all(n <= CAPACITY for _, _, n in d.trace)
        print(logical_size, d.name, 'frames', len(d.trace), 'bytes', expected_bytes, 'completion_seconds', d.complete_at)


def main():
    assert CAPACITY == 146
    assert SYNC['items_each_direction'] == 8 and SYNC['page_items'] == 4
    for size in (338, 443, 556, SYNC['logical_size_bytes']):
        witness(size)
    print('Setup per direction: 9 frames, 1227 bytes; proof frames are reserved capacity, not valid proof fixtures.')
    print('Worksheet passes; no production codec, simulation, cryptographic or hardware result claimed.')


if __name__ == '__main__':
    main()

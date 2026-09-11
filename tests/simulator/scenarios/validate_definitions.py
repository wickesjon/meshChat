"""Validate the predeclared scenario inputs; does not run a mesh simulator."""
from collections import deque
from hashlib import sha256
import json
from pathlib import Path


def positions(encoded_id):
    raw = bytes.fromhex(encoded_id)
    assert len(raw) == 8 and raw.hex() == encoded_id
    digest = sha256(b'meshfest-bloom-v1' + raw).digest()
    first = int.from_bytes(digest[:8], 'big')
    step = int.from_bytes(digest[8:16], 'big')
    return [(first + index * step) % 4096 for index in range(6)]


def validate_selection_fixtures(path):
    fixtures = json.loads(path.read_text(encoding='utf-8'))
    assert fixtures['fixture_version'] == 1
    assert fixtures['bloom']['bits'] == 4096 and fixtures['bloom']['hashes'] == 6
    for kind in ('joined_private', 'absent_private'):
        channel = fixtures['channels'][kind]
        assert sha256(('meshfest-v1|' + channel['name']).encode()).hexdigest()[:8] == channel['id']
    for case in fixtures['cases']:
        held = case.get('requester_held_ids')
        if held is None:
            start, end = case['requester_held_u64_range_inclusive']
            held = [f'{n:016x}' for n in range(start, end + 1)]
        assert len(held) == len(set(held)) == case['request_item_count']
        expected_filter = bytes.fromhex(case['request_bloom_hex'])
        computed = bytearray(512)
        for msg_id in held:
            for bit in positions(msg_id):
                computed[bit // 8] |= 1 << (7 - bit % 8)
        assert len(expected_filter) == 512 and computed == expected_filter
        selected, known, false_positive, budget = [], [], [], []
        selected_bytes = 0
        for record in case['responder_newest_first']:
            msg_id = record['msg_id']
            assert record['type'] == 'CHAT' and record['ttl'] == 0
            assert len(bytes.fromhex(record['channel_id'])) == 4
            assert record['logical_size_bytes'] == 26 + 4 + 1 + 1 + 20 + 4 + 2 + 280
            present = all(expected_filter[p // 8] & (1 << (7 - p % 8)) for p in positions(msg_id))
            if present:
                (known if msg_id in held else false_positive).append(msg_id)
            elif len(selected) == fixtures['common']['session_items'] or selected_bytes + record['logical_size_bytes'] > fixtures['common']['session_encoded_bytes']:
                budget.append(msg_id)
            else:
                selected.append(msg_id)
                selected_bytes += record['logical_size_bytes']
        assert selected == case['expected_selected_ids']
        assert selected_bytes == case['expected_selected_encoded_bytes']
        assert known == case['expected_bloom_known_omissions']
        assert false_positive == case['expected_bloom_false_positive_omissions']
        assert budget == case['expected_budget_omissions']
        by_id = {r['msg_id']: r for r in case['responder_newest_first']}
        assert len(by_id) == len(case['responder_newest_first'])
        displayed = [i for i in selected if by_id[i]['channel_id'] in case['requester_subscriptions']]
        assert displayed == case['expected_subscription_display_ids']
        assert [i for i in selected if i not in displayed] == case['expected_unsubscribed_but_received_ids']
        page_size = fixtures['common']['page_items']
        pages = [selected[i:i + page_size] for i in range(0, len(selected), page_size)]
        assert pages == case['expected_page_data_ids']
        assert [3] * (len(pages) - 1) + [7 if budget else 5] == case['expected_page_marker_flags']
        assert case['expected_forwarded_ids'] == []  # Every stored fixture has TTL zero.
        if 'repeat' in case:
            repeat = case['repeat']
            assert repeat['session_ids'] == [1, 2] and repeat['request_start_seconds'] == [0, 60]
            assert repeat['expected_selected_ids_each_walk'] == [selected, selected]
            assert repeat['expected_false_positive_omissions_each_walk'] == [false_positive, false_positive]
            assert repeat['expected_new_display_ids_each_walk'] == [displayed, []]
            missing = {r['msg_id'] for r in case['responder_newest_first']} - set(held)
            assert len(missing) == repeat['missing_before_first_walk']
            assert len(set(selected)) == repeat['recovered_distinct_after_both_walks']
            assert len(selected) == repeat['selected_set_denominator_each_walk']
        print(f"{case['id']}: {len(selected)} selected, {len(false_positive)} false-positive omissions; fixed filter/outcomes pass")


def validate(path):
    data = json.loads(path.read_text(encoding='utf-8'))
    assert data['schema_version'] == 1 and data['evidence_state'] == 'specified'
    assert len(set(data['seeds'])) == len(data['seeds']) > 0
    assert all(type(s) is int and 0 <= s < 2**64 for s in data['seeds'])
    ttl = data['live_workload']['ttl']
    times = data['live_workload']['origin_times_seconds']
    assert times == sorted(set(times))
    assert all(0 <= t < data['live_workload']['duration_seconds'] for t in times)
    summaries = []
    for topology in data['topologies']:
        count = topology['nodes']
        edges = [tuple(sorted(e)) for e in topology['edges']]
        assert len(set(edges)) == len(edges)
        neighbors = [set() for _ in range(count)]
        for u, v in edges:
            assert 0 <= u < v < count
            neighbors[u].add(v)
            neighbors[v].add(u)
        assert max(map(len, neighbors)) <= 6, 'Normal link ceiling'
        for node in topology.get('saver_repeat_nodes', []):
            assert len(neighbors[node]) <= 3
        for edge in topology.get('bridge_edges', []):
            assert tuple(sorted(edge)) in edges
        reachable = excluded = 0
        for index in range(len(times)):
            origin = index % count
            distance = {origin: 0}
            queue = deque([origin])
            while queue:
                u = queue.popleft()
                for v in sorted(neighbors[u]):
                    if v not in distance:
                        distance[v] = distance[u] + 1
                        queue.append(v)
            assert len(distance) == count, 'All declared static overlays must be connected'
            reachable += sum(1 <= d <= ttl for d in distance.values())
            excluded += sum(d > ttl for d in distance.values())
        assert reachable + excluded == len(times) * (count - 1)
        summaries.append((topology['id'], reachable, excluded))
    sync = data['sync']
    assert sync['items_each_direction'] * sync['logical_size_bytes'] == sync['max_selected_encoded_bytes'] == 8192
    assert sync['page_items'] <= sync['items_each_direction'] == 8
    assert sync['pass']['completion_seconds_max'] == sync['duration_seconds'] == 120
    assert set(data['capacity_cases']) == {145, 146, 182, 512}
    assert data['adversarial']['attacker_links'] <= 8
    fixture_path = path.parent / sync['fixed_selection_fixtures']
    assert fixture_path.resolve().parent == path.parent.resolve()
    validate_selection_fixtures(fixture_path)
    for name, reachable, excluded in summaries:
        print(f'{name}: {reachable} scheduled TTL-reachable recipient pairs, {excluded} beyond TTL')
    print('Scenario definition consistency passes; no simulated delivery/latency result claimed.')


if __name__ == '__main__':
    validate(Path(__file__).with_name('MC-007-acceptance.json'))

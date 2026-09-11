"""Validate the predeclared scenario inputs; does not run a mesh simulator."""
from collections import deque
import json
from pathlib import Path


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
    for name, reachable, excluded in summaries:
        print(f'{name}: {reachable} scheduled TTL-reachable recipient pairs, {excluded} beyond TTL')
    print('Scenario definition consistency passes; no simulated delivery/latency result claimed.')


if __name__ == '__main__':
    validate(Path(__file__).with_name('MC-007-acceptance.json'))

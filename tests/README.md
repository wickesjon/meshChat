# Tests and evidence

`ticketboard/validate.py` checks the planning graph and generated index. Future tickets add the simulator, golden vectors, integration/fuzz/adversarial suites, and physical bench/field procedures described in [the plan](../docs/ticketboard/implementation-plan.md). Run `python -B tests/ticketboard/validate.py` and `python -B -m unittest discover -s tests/ticketboard -v` from the repository root. Physical measurements and independent reviews must be recorded as evidence; simulation is not a substitute.

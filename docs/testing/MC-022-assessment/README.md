# Original MC-022 automated assessment evidence

The [report](report.md) is the assessor's unchanged report on candidate `ad87cf890459cd43e1e10991085440ebd6d48cb9`, production `835a7966f4edb2dead99f5768ec24b75059a337c`. Its open finding statuses describe that original revision, not later remediation. The [approved remediation record](../MC-022-assessment-remediation.md) and MC-022 ticket track subsequent disposition/retest.

This was an independently executed construction/transcript assessment by a separately assigned AI worker. It is not an external human audit, formal proof or device certification. Ordinary Terra PR review remains separate.

Inventories, original report bytes, defect-reproducing patches, logs, dependency graph/features and the input audit script are retained as received in [evidence.zip](evidence.zip). The binary archive preserves their byte-level hashes and patch context whitespace independently of Git's text line-ending conversion. The readable report alongside it has the same content. Absolute paths identify the original local run; ignored source/build directories are reproducible from the recorded Git revisions and are not committed. The snapshot used Git archive's CRLF working-tree conversion: all 233 files match the candidate after documented CRLF-to-LF normalization, and actual input SHA-256 values are retained. Probe patches target a disposable copy and intentionally assert the defective behavior; green original probes reproduce flaws, not security acceptance.

Reproduction and tool versions are documented in the report. The `audit_inputs.py` script is the original run artifact and contains its explicit local workspace paths. The normative tests added during remediation assert the corrected behavior instead of those historical defect assertions. Existing report limitations, including dated advisory data and incomplete errata retrieval, remain visible.

## Remediation retest

The same separately assigned assessor independently retested revision `695fbc44e472e1f6a5664c9ec0cb3b8a0cc8f792` and supplied the unchanged [retest report](retest-report.md): **F-01, F-02 and A-01 resolved; no new blocking construction finding**. Eight independent probes, 63 committed integration checks and 13 storage-policy checks passed. A-02 remains a native integration obligation, not a claim of finished native features. Physical/native execution and MC-037 remain separate gates.

[Retest evidence](retest-evidence.zip) preserves original report bytes, source inventory, independent probe patch, modification hashes, scripts and logs, plus the assessor's output inventory. Every inventory hash was checked before archiving. The inventory also identifies `retest-source.tar`; that reproducible source archive is intentionally omitted from the ZIP, since it is generated from the exact Git revision. The 238-file snapshot matches its explicit Git objects after documented CRLF normalization (236 text transformations). Original assessment artifacts are unchanged.

- Original retest report SHA-256: `bd83215580ff5a0de0eb98de40361b47c90068ba714c12c0bcd376ac4296fe2f`.
- Retest evidence ZIP SHA-256: `03925e6866b68cbb272ff253a63fd271be3e293cc92af43130820c66e1fd7765`.

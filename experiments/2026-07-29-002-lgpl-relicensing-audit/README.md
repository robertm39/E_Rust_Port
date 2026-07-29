# LGPL-3.0 relicensing readiness audit

Bead: `E_Rust_Port-9jt.8.8`

This investigation audits the exact distributable boundary produced by the
clean Ubuntu 24.04 package gate for `E_Rust_Port-9jt.4.8`. It is an engineering
and provenance record, not legal advice. It does not change Umlaut's current
`GPL-2.0-or-later` declaration.

## Frozen inputs

- repository commit before audit-only files:
  `96561cd3` (`feat(sat): add optional static CaDiCaL service`);
- source package: `umlaut-0.1.0-source.tgz`, 314 members, SHA-256
  `2d82e62955b0f2eb1a9a1c2c77007e05fefc3af0c4130aee83618416664a5b3f`;
- StarExec package: `umlaut-0.1.0-starexec.tgz`, five members, SHA-256
  `e79448ef845c83e1f7022a2b9b12949a16db722812862a15e104526197c687a3`;
- E source and verbatim-data revision:
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`;
- CaDiCaL optional build-input revision:
  `c60730422e758ef1cebe7aeddf2dda31c996bf04`.

The source package predates the audit prose that recorded its own hash.
`audit_inventory.py` therefore retains each archived hash and also hashes the
corresponding canonical Git `HEAD` blob. It records both exact byte equality
and equality after the sole allowed normalization, CRLF to LF. It fails on an
unclassified package member or unsafe archive path.

## Authoritative references

- E's pinned `COPYING` states that E versions from 1.2 are offered under
  GPL-2.0-or-later and LGPL-2.1-or-later:
  <https://github.com/eprover/eprover/blob/17026b1bfe61aaf223cfaae54947c8d2679c31a0/COPYING>
- SPDX distinguishes `LGPL-2.1-or-later` from `LGPL-2.1-only`:
  <https://spdx.org/licenses/LGPL-2.1-or-later.html>
- The FSF says a project available under LGPL-2.1-or-later can select LGPL-3.0
  or later:
  <https://www.gnu.org/licenses/gpl-faq.html#AllCompatibility>
- LGPL-3.0 text:
  <https://www.gnu.org/licenses/lgpl-3.0.html>
- OpenAI's terms assign any OpenAI interest in Output to the user as between
  those parties, while warning that output may not be unique and that the user
  remains responsible for rights and accuracy:
  <https://openai.com/policies/row-terms-of-use/>

These references establish technical evidence only. They do not prove
copyright ownership, employment rights, absence of third-party similarity, or
the legal effect of relicensing this particular codebase.

## Reproduction

Extract the two inner package archives from the ignored final evidence bundle,
then run:

```text
python experiments/2026-07-29-002-lgpl-relicensing-audit/audit_inventory.py \
  --source-archive SOURCE_TGZ \
  --runtime-archive STAREXEC_TGZ \
  --e-source eprover \
  --output-dir experiments/2026-07-29-002-lgpl-relicensing-audit
python -m unittest \
  experiments/2026-07-29-002-lgpl-relicensing-audit/test_audit_inventory.py
```

The checked-in `source-members.jsonl`, `runtime-members.jsonl`,
`e-source-headers.jsonl`, and `summary.json` are the immutable results.
`FINDINGS.md` records the decision and exact human/legal follow-up.

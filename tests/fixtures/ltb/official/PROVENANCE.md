# Official CASC LTB batch fixtures

Retrieved from the official TPTP/CASC site on 2026-07-16 for permanent parser
compatibility tests. The checked-in files are byte-for-byte copies; the
repository line-ending rule keeps them LF-stable across platforms.

| Fixture | Official URL | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `BatchSampleLTBJJT.txt` | <https://tptp.org/CASC/28/LTBExamples/BatchSampleLTBJJT.txt> | 2,850 | `e85f9ccff5b281b6adf96fa3d4f4467c849a67e5d32432e80deeb0b45e661083` |
| `BatchSpec.VBT.txt` | <https://tptp.org/CASC/J11/Examples/BatchSpec.VBT.txt> | 29,751 | `14f505ac10d1782187bb20f8a82bb97bd73d8f47e07c728aeba1910d037bd295` |
| `BatchSampleLTBHLL` | <https://tptp.org/CASC/J8/BatchSampleLTBHLL> | 1,410 | `36013f79ea453cbc3bed7230f1a8728e9d3e2e2603f2f449144f3f03b7466f96` |

The CASC-28 JJT and CASC-J11 VBT fixtures use the `training_data` spelling
accepted by the current upstream E runner. The older sample served through the
CASC-J8 link identifies `LTB.HOL` and uses `training_directory`; current E has
that spelling commented out in `PROVER/e_ltb_runner.c` and therefore rejects
it. Tests preserve both the accepted modern corpus and this source-confirmed
historical boundary.

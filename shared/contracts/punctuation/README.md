# Punctuation contract

`policy.json` is the authoritative list of ASCII punctuation keys handled by Chinese punctuation mode. `generate.py` emits `policy.h`; consumers use the generated header or, in Rust, `crates/engine/src/punctuation.rs`, whose `matches_the_shared_contract` test compares its table with both files. Nobody copies the mapping into a platform-specific table.

The contract holds stateless simple mappings plus the opening/closing pairs for alternating quotes and nested book-title marks. The quote toggles and nesting depth are session state, so each input session owns that state; the contract provides no shared mutable state.

After editing `policy.json`, run `python3 shared/contracts/punctuation/generate.py` from the repository root. `scripts/test-punctuation-contract.py` runs it with `--check` in every `scripts/run-checks.sh`, so a header that does not match the JSON fails the gate. The C++ contract test (`shared/contracts/tests/punctuation_contract.cpp`) is built by the Windows project.

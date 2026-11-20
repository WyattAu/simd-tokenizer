# Pattern adoption

Declared per the pattern canon adoption format; every claim is verified against code in this repo.

| Pattern | Status | Evidence |
|---|---|---|
| T40 | partial | src/swar.rs:50 (borrowed byte slices, zero allocation per count, see PERF-SLO.md); a counting kernel, not wire framing |
| T1 | n/a | single-pass byte-run counter; no validate-then-reparse surface |

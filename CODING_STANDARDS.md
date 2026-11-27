# Coding standards

Read during review. Mechanical rules live in `scripts/check.sh` and `scripts/release.sh`; this file holds the judgement calls.

## Public types

A new public struct or enum that quando hands back (`Found`, `Parsed`, `SpanKind`) is `#[non_exhaustive]`, so a later field or variant is not a breaking change. A type the caller builds with a struct literal (`DateTime`, `Options`) stays open; adding a field to one is breaking, and `release.sh` makes that a minor bump in 0.x.

## Parse results

A diff that changes an existing `want` in `tests/cases.json` changes what users get back. The commit message says so, since it makes the next release a breaking one.

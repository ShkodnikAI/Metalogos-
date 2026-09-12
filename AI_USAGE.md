# Use of Generative AI in Metalogos Development

This document describes, in general terms, how generative AI (GenAI) tools have
been used in the development of Metalogos, in accordance with our commitment to
transparency and with NLnet's Policy on the use of Generative Artificial
Intelligence (https://nlnet.nl/foundation/policies/generativeAI/).

## Summary

Metalogos has been developed using a structured, contract-first workflow in
which an AI coding assistant (Claude, by Anthropic) is used substantively for
writing implementation code, tests, and documentation, under the direction and
review of the project owner (Siarhei Lakhanski). The owner defines scope,
architecture decisions, and acceptance criteria; the AI assistant implements
against those criteria; every change is verified against real compiled code
and a continuous integration pipeline before being merged, not accepted on
the AI's own claim of correctness.

## How GenAI is used

- **Code generation**: a substantial portion of the Rust implementation,
  `.mlog` example programs, and test suites has been written by the AI
  assistant, working from written task specifications ("naryads") that
  define the contract (a minimal example program plus its expected output),
  the scope of what should and should not be touched, and any relevant
  architectural precedent to follow.
- **Architecture decisions**: significant design decisions (type system
  choices, security model, module boundaries) are documented as Architecture
  Decision Records (ADRs) in `docs/adr/`, including decisions the project
  chose not to pursue and why. Semantic decisions require this documented
  prior-art-and-alternatives process before implementation; purely mechanical
  extensions of existing patterns do not.
- **Verification**: every change is independently checked against the actual
  compiled code, the real continuous integration results, and the stated
  task contract before being merged — not accepted based on the AI's own
  report of what it did. This verification step has caught and required
  correction of real defects (see the project's `CHANGELOG.md` and the
  naryad history in `docs/adr/` for concrete examples where AI-produced work
  was found incomplete or incorrect and was fixed before being accepted).
- **Documentation and translation**: project documentation, including this
  file, has been drafted with AI assistance and reviewed by the project
  owner.

## What is not done

- Generated code, documentation, or other artifacts are not merged into the
  project without human review and without passing the project's automated
  test suite and security audit checks.
- The AI assistant does not have unsupervised write access to the public
  repository; changes are proposed, tested, and merged by the project owner
  or under the owner's explicit direction.

## Going forward

For work carried out after this disclosure is published, commits that
substantially consist of AI-generated code will identify the model used in
the commit message, following the pattern recommended by NLnet's policy.

## Licensing

The project owner takes responsibility for ensuring that AI-assisted output
does not reproduce copyrighted or license-incompatible material, and that all
project code remains correctly licensed under Metalogos's stated open source
licenses (MIT/Apache-2.0).

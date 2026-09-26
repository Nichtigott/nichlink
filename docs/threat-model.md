# Threat Model

[简体中文](threat-model.zh-CN.md) | English

## Assets

- The host registration tree and its parent/contract invariants.
- Plugin bytes, signatures, checksums, lock records, and revocation metadata.
- The host process and data supplied to plugin operations.

## Trust boundaries

Core declarations are metadata and do not execute plugin code. Studio and the
debug collector are development tools. Wasm and process adapters are separate
execution boundaries; process adapters are additionally isolated by a fresh
child process per call.

## Controls

- Release builds consume a checked `StaticPlan` and do not retain inventory
  linker sections.
- External faces are validated for provenance, framework, version, parent, and
  contract closure before publication.
- Official plugins require a matching trust policy and signature assurance;
  community and local plugins still require a verified checksum artifact. The host
  path that applies this is `PluginAdmission` in `nichlink-plugin-host`: it reads
  the plugin locks, refuses an unlisted official plugin before any verifier runs,
  and refuses a revoked version before the signature. The signature covers the
  registration that travels with the plugin bytes, not only the manifest.
- Wasm calls are limited by linear memory, table elements, fuel, input bytes,
  output bytes, and the artifact size accepted for compilation; modules are also
  compiled under the engine's strict limits.
- Process calls use framed I/O, input/output limits, and a hard timeout; a
  timed-out child is terminated.
- Hot replacement publishes code and registry in one generation. Failed health
  checks or graft validation leave the previous generation live.
- Revocation and flow-version mismatches reject a candidate before activation.

## Residual risk

The core cannot prove a complete static call graph for arbitrary Rust features,
including dynamic dispatch, function pointers, FFI, and optimized-away locals.
MIR candidates and unobserved values are labeled as evidence with conservative
fallbacks. Wasm memory limits do not make a malicious plugin logically safe;
applications must still choose contracts and capabilities appropriate to their
data.

A **process** plugin is the weakest boundary here: the host frames its I/O,
bounds input, output, and wall time, and terminates a child that overruns, but
the child inherits the host's environment, working directory, filesystem access,
and network access. The README's plugin section says so and disclaims a sandbox;
this paragraph is the residual-risk half of that statement. A host that runs
untrusted process plugins must confine the child outside NichLink — a container,
a user, or an OS sandbox — because nothing in this workspace does it.

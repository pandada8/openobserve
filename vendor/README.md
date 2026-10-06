# Dependency security patches

`vrl/` is the crates.io source distribution of VRL 0.31.0 (MPL-2.0),
upstream commit `4d61e53334cb373b1cf5129d3963145bd5bff6d0`.
The original crate SHA-256 is
`398a6efaf0e120eae12249f7cc89a82ffd3fa8d98165e341bee6dd9a973bba6a`.
Its license and upstream source files are retained.

The only functional patch is changing `domain` from `0.11.0` to `0.12.3`
in both Cargo manifests to fix RUSTSEC-2026-0310. The root Cargo patch
also applies to Vector's VRL dependency, retaining the shared 0.31 API.
No DNS functions are disabled and no advisory is ignored.

`parse_cbor` also uses ordinary slice indexing instead of the `WriteBuf::as_slice`
extension method to avoid a nightly `unstable_name_collisions` error exposed by
compiling VRL as a local dependency. This preserves the same bytes passed to CBOR.

Cargo's registry bookkeeping and the standalone crate lockfile are omitted;
the application root `Cargo.lock` is authoritative. Remove this vendor patch
when the application's VRL and Vector dependencies can jointly use an upstream
release with the fixed DNS dependency.

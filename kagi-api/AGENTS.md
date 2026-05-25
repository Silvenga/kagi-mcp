# kagi-api Crate

## OVERVIEW
Kagi Search/Extract HTTP API client library. Flat public API with builder-pattern configuration.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add request type | `src/types/search_request.rs` | Builder methods, derives |
| Add response type | `src/types/search_response.rs` | Deserialize, optional fields |
| Error mapping | `src/error.rs` | Map HTTP status → `KagiError` variant |
| Client customization | `src/builder.rs` | `with_*` methods |
| Mock for tests | `src/api_trait.rs` | `KagiApi` trait + `mockall` |

## CONVENTIONS
- Flat re-exports: everything public lives in crate root
- `KagiClientBuilder` produces `KagiClient` implementing `KagiApi` trait
- `async-trait` macro for the `KagiApi` trait (mockable via `#[cfg(feature = "mock")]`)
- All request types use builder pattern (`with_*` returning `Self`)

## ANTI-PATTERNS
- Never import from `kagi_api::client` or `kagi_api::types` — use root
- Test-only constructors (e.g., `open_in_memory`) → use `#[cfg(test)]` visibility

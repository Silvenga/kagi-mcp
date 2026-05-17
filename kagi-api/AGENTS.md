# kagi-api Crate Conventions

## Public API Surface

This crate uses a **flattened public API**. All types, traits, and structs are re-exported directly from the crate root. Consumers should always import from `kagi_api` directly, never from submodules.

### Correct

```rust
use kagi_api::{KagiClient, KagiClientBuilder, SearchRequest, KagiApi};
```

### Incorrect

```rust
use kagi_api::client::KagiClient;           // module path — private
use kagi_api::types::SearchRequest;         // module path — private
use kagi_api::error::KagiError;             // module path — private
```

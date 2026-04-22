# omada-client

A Rust client library for the TP-Link Omada REST API.

## Project Overview

- **API spec**: `oas.json` (OpenAPI 3.0.1 — Omada Open API v0.1)
- **Scope**: Only a subset of endpoints from the spec will be implemented; add methods as needed rather than generating the full spec
- **HTTP client**: `reqwest` with `tokio` async runtime (use the `async`/`await` style throughout)

## Development Commands

```sh
cargo build
cargo test
cargo clippy -- -W clippy::pedantic
```

Run `cargo clippy -- -W clippy::pedantic` after each change. Clippy warnings are not gospel — suppress them with `#[allow(...)]` when they conflict with readability or the Rust API Guidelines. Prefer idiomatic, readable code over mechanical lint compliance.

## Architecture

- All public API methods live on a central `OmadaClient` struct in `src/lib.rs` (or submodules under `src/`)
- The client holds the base URL, `omadacId`, and an authenticated `reqwest::Client`
- Public request/response types (returned to API consumers) are defined in `src/models.rs` and re-exported at the crate root via `pub use models::*`
- Internal-only types (e.g. auth request bodies, token structs) stay in `src/lib.rs` alongside the code that uses them
- Each client instance will have an access token.  The client is responsible for managing the lifetime of this token.  Locking and update strategies should minimize contention and favor performance.

## Method Naming

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html):

| Pattern | Convention |
|---|---|
| Getters | `field_name()` — no `get_` prefix (C-GETTER) |
| Conversions to owned | `to_foo()` |
| Conversions borrowing | `as_foo()` |
| Consuming conversions | `into_foo()` |
| Casing | `snake_case` for functions/methods, `CamelCase` for types (C-CASE) |
| Constructors | Associated `new` / `with_*` methods, no bare functions (C-CTOR) |
| Word order | Object–verb–qualifier, e.g. `sites_list`, `device_reboot` (C-WORD-ORDER) |

Map OAS `operationId` camelCase names to idiomatic `snake_case`, dropping redundant words:

| OAS operationId | Rust method name |
|---|---|
| `getUser` | `user()` |
| `modifyUser` | `update_user()` |
| `deleteUser` | `delete_user()` |
| `getSiteTemplateEntity` | `site_template()` |
| `updateSiteTemplateEntity` | `update_site_template()` |

## Dependencies to Add

This is a library crate — only specify the minimal features actually needed. Do **not** use catch-all feature flags like `tokio/full`; let consumers choose their runtime configuration.

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

## Error Handling

- Define a crate-level `Error` enum using `thiserror`
- Wrap `reqwest::Error` and API-level errors (non-2xx responses with Omada error body)
- Return `Result<T, crate::Error>` from all public async methods

## URL Structure

All endpoints follow the pattern:

```
{base_url}/openapi/{version}/{omadacId}/sites/{siteId}/...
```

Store `base_url` and `omadacId` on the client. Pass `site_id` as a parameter to methods that require it.  Version can be hardcoded.

## Testing

- Use [`wiremock`](https://crates.io/crates/wiremock) for all HTTP-level tests. Spin up a `MockServer`, mount `Mock` handlers that assert the exact method, path, query params, headers, and JSON body prescribed by the spec, then assert on the response values.
- Tests live in a `#[cfg(test)]` module inside the relevant source file.
- Each auth grant type (client credentials, authorization code, refresh token) must have its own test that verifies spec compliance.
- Error paths (e.g. non-zero `errorCode` body) must also be covered.

```toml
[dev-dependencies]
wiremock = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Code Style

- `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]` on all request/response structs
- `#[serde(rename_all = "camelCase")]` to match the JSON field casing in the API
- Use `Option<T>` for fields marked optional in the spec
- Avoid `unwrap()`/`expect()` in library code; propagate errors with `?`
- Every public field on a public type must have a `///` doc comment sourced from the spec's `description`. Lightly edit for Rust doc conventions (backtick literals, wrap at ~100 chars) but preserve the original meaning.

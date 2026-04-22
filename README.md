# omada-client

An async Rust client for the [TP-Link Omada Open API](https://www.tp-link.com/us/support/download/omada-software-controller/).
Built on [`reqwest`](https://docs.rs/reqwest) and [`tokio`](https://tokio.rs).

## Installation

```toml
[dependencies]
omada-client = "0.1"
```

By default the `serde_json` feature is enabled. To use `sonic-rs` instead:

```toml
omada-client = { version = "0.1", default-features = false, features = ["sonic-rs"] }
```

## Authentication

The Omada Open API supports three grant types. Pick the one that matches your
application registration.

### Client credentials

Used for server-to-server integrations where a user login is not required.

```rust,no_run
#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let client = omada_client::OmadaClient::with_client_credentials(
        "https://omada.example.com",
        "abc123omadacid",
        "my-client-id",
        "my-client-secret",
    )
    .await?;

    // client is ready to use
    Ok(())
}
```

### Authorization code (via login)

Used when the Omada application is not configured with a redirect URL.

```rust,no_run
#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let session = omada_client::OmadaClient::login(
        "https://omada.example.com",
        "abc123omadacid",
        "my-client-id",
        "admin",
        "password",
    )
    .await?;

    let code = session.authorize_code().await?;

    let client = omada_client::OmadaClient::with_authorization_code(
        "https://omada.example.com",
        "abc123omadacid",
        "my-client-id",
        "my-client-secret",
        &code,
    )
    .await?;

    Ok(())
}
```

### Self-signed certificates

To connect to a controller with a self-signed certificate, use
`OmadaClient::builder()`:

```rust,no_run
#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let client = omada_client::OmadaClient::builder()
        .danger_accept_invalid_certs(true)
        .with_client_credentials(
            "https://192.168.1.1",
            "abc123omadacid",
            "my-client-id",
            "my-client-secret",
        )
        .await?;

    Ok(())
}
```

## Usage examples

### List all sites

```rust,no_run
use futures_util::TryStreamExt as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let client = omada_client::OmadaClient::with_client_credentials(
        "https://omada.example.com", "abc123", "id", "secret",
    ).await?;

    let sites: Vec<_> = client.sites(None).try_collect().await?;
    for site in &sites {
        println!("{}: {}", site.site_id, site.name);
    }
    Ok(())
}
```

### Read and update an AP's general config

```rust,no_run
#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let client = omada_client::OmadaClient::with_client_credentials(
        "https://omada.example.com", "abc123", "id", "secret",
    ).await?;

    let site_id = "site-abc";
    let ap_mac = "AA-BB-CC-DD-EE-FF";

    let mut cfg = client.ap_general_config(site_id, ap_mac).await?;
    cfg.name = Some("lobby-ap".to_owned());
    cfg.led_setting = Some(1); // on
    client.update_ap_general_config(site_id, ap_mac, &cfg).await?;

    Ok(())
}
```

### Create an SSID

```rust,no_run
use omada_client::{CreateSsidRequest, SsidPskSetting};

#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let client = omada_client::OmadaClient::with_client_credentials(
        "https://omada.example.com", "abc123", "id", "secret",
    ).await?;

    let site_id = "site-abc";
    let wlan_id = "wlan-abc";

    client.create_ssid(site_id, wlan_id, &CreateSsidRequest {
        name: "CorpWifi".to_owned(),
        device_type: 1,  // gateway
        band: 7,         // 2.4 + 5 + 6 GHz
        security: 3,     // WPA-Personal
        broadcast: true,
        psk_setting: Some(SsidPskSetting {
            security_key: Some("s3cr3tpassword".to_owned()),
            version_psk: 2,     // WPA2-PSK
            encryption_psk: 3,  // AES
            ..Default::default()
        }),
        ..Default::default()
    }).await?;

    Ok(())
}
```

### Update a switch port profile

```rust,no_run
use omada_client::SwitchPortConfig;

#[tokio::main(flavor = "current_thread")]
async fn main() -> omada_client::Result<()> {
    let client = omada_client::OmadaClient::with_client_credentials(
        "https://omada.example.com", "abc123", "id", "secret",
    ).await?;

    let site_id = "site-abc";
    let switch_mac = "AA-BB-CC-00-11-22";
    let port = "3";

    client.update_switch_port(site_id, switch_mac, port, &SwitchPortConfig {
        profile_id: Some("profile-id".to_owned()),
        ..Default::default()
    }).await?;

    Ok(())
}
```

## Logging

Request and response details (URL, headers, body) are emitted at the `DEBUG`
log level using the [`log`](https://docs.rs/log) crate. Enable them with any
`log`-compatible subscriber such as `env_logger`:

```sh
RUST_LOG=omada_client=debug cargo run
```

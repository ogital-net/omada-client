//! Fetches and prints the radio configuration for a single AP.
//!
//! # Usage
//!
//! ```sh
//! cargo run --example ap_radio_config -- <site-name> <ap-mac>
//! ```
//!
//! # Required environment variables
//!
//! These can be set in the environment or in a `.env` file in the working
//! directory:
//!
//! | Variable              | Description                            |
//! |-----------------------|----------------------------------------|
//! | `OMADA_BASE_URL`      | Controller URL, e.g. `https://omada.example.com` |
//! | `OMADA_OMADAC_ID`     | The `omadacId` for your controller     |
//! | `OMADA_CLIENT_ID`     | Open API application client ID         |
//! | `OMADA_CLIENT_SECRET` | Open API application client secret     |

use futures_util::TryStreamExt as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    // Load .env if present; silently ignore a missing file.
    let _ = dotenvy::dotenv();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <site-name> <ap-mac>", args[0]);
        std::process::exit(1);
    }
    let site_name = &args[1];
    let ap_mac = &args[2];

    let base_url = std::env::var("OMADA_BASE_URL")?;
    let omadac_id = std::env::var("OMADA_OMADAC_ID")?;
    let client_id = std::env::var("OMADA_CLIENT_ID")?;
    let client_secret = std::env::var("OMADA_CLIENT_SECRET")?;

    let client = omada_client::OmadaClient::builder()
        .danger_accept_invalid_certs(true)
        .with_client_credentials(base_url, omadac_id, client_id, client_secret)
        .await?;

    // Find the site whose name matches the argument; take the first result.
    let site = client
        .sites(Some(site_name.clone()))
        .try_next()
        .await?
        .ok_or_else(|| format!("no site found matching {site_name:?}"))?;

    println!("site: {} ({})", site.name, site.site_id);

    let cfg = client.ap_radio_config(&site.site_id, ap_mac).await?;

    print_band("2.4 GHz", cfg.radio_setting2g.as_ref());
    print_band("5 GHz  ", cfg.radio_setting5g.as_ref());
    print_band("5 GHz-1", cfg.radio_setting5g1.as_ref());
    print_band("5 GHz-2", cfg.radio_setting5g2.as_ref());
    print_band("6 GHz  ", cfg.radio_setting6g.as_ref());

    Ok(())
}

fn print_band(label: &str, setting: Option<&omada_client::ApRadioSetting>) {
    let Some(r) = setting else { return };
    println!(
        "{label}: enabled={:?}  channel={:?}  width={:?}  tx_power={:?}  mode={:?}",
        r.radio_enable, r.channel, r.channel_width, r.tx_power, r.wireless_mode,
    );
}

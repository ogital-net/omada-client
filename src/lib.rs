#![doc = include_str!("../README.md")]

#[cfg(all(feature = "serde_json", feature = "sonic-rs"))]
compile_error!("features `serde_json` and `sonic-rs` are mutually exclusive; enable only one");
#[cfg(all(feature = "serde_json", feature = "simd-json"))]
compile_error!("features `serde_json` and `simd-json` are mutually exclusive; enable only one");
#[cfg(all(feature = "sonic-rs", feature = "simd-json"))]
compile_error!("features `sonic-rs` and `simd-json` are mutually exclusive; enable only one");
#[cfg(not(any(feature = "serde_json", feature = "sonic-rs", feature = "simd-json")))]
compile_error!(
    "at least one of the `serde_json`, `sonic-rs`, or `simd-json` features must be enabled"
);

// ── JSON backend shim ─────────────────────────────────────────────────────────

/// Active JSON backend, selected at compile time by the `serde_json`, `sonic-rs`,
/// or `simd-json` feature flag.
#[cfg(feature = "serde_json")]
mod json {
    pub use serde_json::{from_slice, to_vec, Error, Value};
}
#[cfg(feature = "sonic-rs")]
mod json {
    pub use sonic_rs::{from_slice, to_vec, Error, Value};
}
#[cfg(feature = "simd-json")]
mod json {
    pub use simd_json::{to_vec, Error, OwnedValue as Value};
    pub fn from_slice<T>(input: &[u8]) -> Result<T, Error>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let mut bytes = input.to_vec();
        simd_json::serde::from_slice(&mut bytes)
    }
}

/// The JSON value type provided by the active JSON backend
/// (`serde_json::Value` by default, `sonic_rs::Value` with `sonic-rs`, or
/// `simd_json::OwnedValue` with `simd-json`).
pub use json::Value as JsonValue;

/// The JSON error type provided by the active JSON backend
/// (`serde_json::Error` by default, `sonic_rs::Error` with `sonic-rs`, or
/// `simd_json::Error` with `simd-json`).
pub use json::Error as JsonError;

use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use futures_util::{stream::BoxStream, StreamExt as _};
use log::{debug, log_enabled, Level};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

pub mod models;
pub use models::*;

pub type Result<T> = std::result::Result<T, Error>;

/// Crate-level error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// A non-zero error code returned in the Omada API response body.
    #[error("API error {error_code}: {msg}")]
    Api { error_code: i32, msg: String },

    /// A JSON deserialization error on the response body.
    #[error("JSON decode error: {0}")]
    Json(#[from] json::Error),
}

// ── Debug-logged HTTP execution ───────────────────────────────────────────────

/// Logs the request line and headers (and body if byte-buffered) at DEBUG level.
fn log_request(req: &reqwest::Request) {
    debug!("--> {} {}", req.method(), req.url());
    for (name, value) in req.headers() {
        debug!("    {}: {}", name, value.to_str().unwrap_or("<binary>"));
    }
    if let Some(bytes) = req.body().and_then(reqwest::Body::as_bytes) {
        if !bytes.is_empty() {
            debug!("    {}", String::from_utf8_lossy(bytes));
        }
    }
}

/// Logs the response status line and headers at DEBUG level.
fn log_response_head(resp: &reqwest::Response) {
    debug!("<-- {}", resp.status());
    for (name, value) in resp.headers() {
        debug!("    {}: {}", name, value.to_str().unwrap_or("<binary>"));
    }
}

/// Extension that replaces `.send().await?.json::<T>().await?` with a single
/// call that additionally logs the full request and response at DEBUG level.
///
/// When the `debug` log level is disabled the hot-path is identical in
/// performance to the plain `reqwest` call: response bytes are collected once
/// and fed to the active JSON backend's `from_slice`, which is exactly what
/// reqwest's own `.json()` does internally (using `serde_json`).
trait RequestBuilderExt {
    /// Serialize `body` with the active JSON backend and attach it as the
    /// request body with `Content-Type: application/json`.
    fn body_json<B: serde::Serialize>(self, body: &B) -> crate::Result<Self>
    where
        Self: Sized;

    async fn send_json<T>(self) -> crate::Result<T>
    where
        T: for<'de> serde::Deserialize<'de>;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    fn body_json<B: serde::Serialize>(self, body: &B) -> crate::Result<Self> {
        let bytes = json::to_vec(body).map_err(crate::Error::Json)?;
        Ok(self.header("Content-Type", "application/json").body(bytes))
    }

    async fn send_json<T>(self) -> crate::Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let do_log = log_enabled!(Level::Debug);

        if do_log {
            // Clone the builder to inspect the request without consuming it.
            // try_clone() returns None only for streaming bodies; all requests
            // in this crate use byte-buffered JSON bodies, so this always succeeds.
            if let Some(snapshot) = self.try_clone() {
                if let Ok(req) = snapshot.build() {
                    log_request(&req);
                }
            }
        }

        let resp = self.send().await?;

        if do_log {
            log_response_head(&resp);
        }

        let bytes = resp.bytes().await?;

        if do_log && !bytes.is_empty() {
            debug!("    {}", String::from_utf8_lossy(&bytes));
        }

        json::from_slice::<T>(&bytes).map_err(crate::Error::Json)
    }
}

// ── API envelope ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    #[serde(rename = "errorCode")]
    error_code: i32,
    msg: String,
    result: Option<T>,
}

impl<T> ApiResponse<T> {
    fn into_result(self) -> Result<T> {
        if self.error_code == 0 {
            self.result.ok_or_else(|| Error::Api {
                error_code: 0,
                msg: "API returned success but no result".to_owned(),
            })
        } else {
            Err(Error::Api {
                error_code: self.error_code,
                msg: self.msg,
            })
        }
    }

    /// Check only the error code; discard any result value.
    ///
    /// Use this for operations whose response body carries no meaningful result
    /// (e.g. create / delete endpoints that return `OperationResponse`).
    fn check(self) -> Result<()> {
        if self.error_code == 0 {
            Ok(())
        } else {
            Err(Error::Api {
                error_code: self.error_code,
                msg: self.msg,
            })
        }
    }
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct LoginBody<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResult {
    csrf_token: String,
    session_id: String,
}

#[derive(Debug, Serialize)]
struct ClientCredentialsBody<'a> {
    #[serde(rename = "omadacId")]
    omadac_id: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
}

/// Body shared by the `authorization_code` and `refresh_token` grant types.
#[derive(Debug, Serialize)]
struct ClientAuthBody<'a> {
    client_id: &'a str,
    client_secret: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResult {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

// ── Token state ───────────────────────────────────────────────────────────────

/// How early to refresh before the token's actual expiry time.
const TOKEN_EXPIRY_BUFFER: Duration = Duration::from_secs(60);

#[derive(Debug)]
struct TokenState {
    access_token: Arc<str>,
    refresh_token: Arc<str>,
    expires_at: Instant,
}

fn token_state_from(r: TokenResult) -> TokenState {
    TokenState {
        expires_at: Instant::now()
            + Duration::from_secs(r.expires_in).saturating_sub(TOKEN_EXPIRY_BUFFER),
        access_token: Arc::from(r.access_token),
        refresh_token: Arc::from(r.refresh_token),
    }
}

// ── ClientBuilder ─────────────────────────────────────────────────────────────

/// Builder for configuring and constructing an [`OmadaClient`] or
/// [`AuthSession`].
///
/// Obtain one via [`OmadaClient::builder`].
///
/// # Example
///
/// ```no_run
/// # async fn example() -> omada_client::Result<()> {
/// let client = omada_client::OmadaClient::builder()
///     .danger_accept_invalid_certs(true)
///     .with_client_credentials("https://omada.example.com", "abc123", "id", "secret")
///     .await?;
/// # Ok(()) }
/// ```
#[derive(Debug, Default)]
pub struct ClientBuilder {
    accept_invalid_certs: bool,
}

impl ClientBuilder {
    /// Creates a new builder with default settings (TLS verification enabled).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Disables TLS certificate verification.
    ///
    /// **Security warning**: disabling certificate verification makes the
    /// connection vulnerable to man-in-the-middle attacks. Only use this when
    /// connecting to a controller that uses a self-signed certificate on a
    /// trusted private network.
    #[must_use]
    pub fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(self.accept_invalid_certs)
            .build()
            .map_err(Error::Http)
    }

    /// Authenticates with the **client credentials** grant and returns a ready
    /// client.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built, the HTTP request
    /// fails, or the API rejects the credentials.
    pub async fn with_client_credentials(
        self,
        base_url: impl Into<String>,
        omadac_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Result<OmadaClient> {
        let base_url = base_url.into();
        let omadac_id = omadac_id.into();
        let client_id = client_id.into();
        let client_secret = client_secret.into();
        let http = self.build_http_client()?;

        let token = OmadaClient::fetch_client_credentials_token(
            &http,
            &base_url,
            &omadac_id,
            &client_id,
            &client_secret,
        )
        .await?;

        Ok(OmadaClient {
            base_url,
            omadac_id,
            client_id,
            client_secret,
            http,
            token: Arc::new(RwLock::new(token)),
        })
    }

    /// Exchanges an authorization code for an access token and returns a ready
    /// client.
    ///
    /// The code is obtained either via the login-page redirect or by calling
    /// [`OmadaClient::login`] followed by [`AuthSession::authorize_code`].
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built, the HTTP request
    /// fails, or the API rejects the code.
    pub async fn with_authorization_code(
        self,
        base_url: impl Into<String>,
        omadac_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        code: impl AsRef<str>,
    ) -> Result<OmadaClient> {
        let base_url = base_url.into();
        let omadac_id = omadac_id.into();
        let client_id = client_id.into();
        let client_secret = client_secret.into();
        let http = self.build_http_client()?;

        let token = OmadaClient::fetch_auth_code_token(
            &http,
            &base_url,
            &client_id,
            &client_secret,
            code.as_ref(),
        )
        .await?;

        Ok(OmadaClient {
            base_url,
            omadac_id,
            client_id,
            client_secret,
            http,
            token: Arc::new(RwLock::new(token)),
        })
    }

    /// Logs in to the Omada controller and returns an [`AuthSession`] that can
    /// be used to obtain an authorization code.
    ///
    /// This is the first step of the authorization-code flow when no redirect
    /// URL is configured on the Open API application.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built, the HTTP request
    /// fails, or the login is rejected.
    pub async fn login(
        self,
        base_url: impl Into<String>,
        omadac_id: impl Into<String>,
        client_id: impl Into<String>,
        username: &str,
        password: &str,
    ) -> Result<AuthSession> {
        let base_url = base_url.into();
        let omadac_id = omadac_id.into();
        let client_id = client_id.into();
        let http = self.build_http_client()?;

        let url = format!("{base_url}/openapi/authorize/login");
        let login = http
            .post(&url)
            .query(&[
                ("client_id", client_id.as_str()),
                ("omadac_id", omadac_id.as_str()),
            ])
            .body_json(&LoginBody { username, password })?
            .send_json::<ApiResponse<LoginResult>>()
            .await?
            .into_result()?;

        Ok(AuthSession {
            base_url,
            omadac_id,
            client_id,
            csrf_token: login.csrf_token,
            session_id: login.session_id,
            http,
        })
    }
}

// ── AuthSession ───────────────────────────────────────────────────────────────

/// An active login session used to obtain an authorization code.
///
/// Returned by [`OmadaClient::login`]. Used in the authorization-code flow
/// when no redirect URL is configured on the Open API application.
#[derive(Debug)]
pub struct AuthSession {
    base_url: String,
    omadac_id: String,
    client_id: String,
    csrf_token: String,
    session_id: String,
    http: reqwest::Client,
}

impl AuthSession {
    /// The CSRF token issued by the login endpoint.
    #[must_use]
    pub fn csrf_token(&self) -> &str {
        &self.csrf_token
    }

    /// The session ID issued by the login endpoint.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Requests an authorization code from the Omada controller.
    ///
    /// The returned code can be passed to [`OmadaClient::with_authorization_code`].
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn authorize_code(&self) -> Result<String> {
        let url = format!("{}/openapi/authorize/code", self.base_url);
        self.http
            .post(&url)
            .query(&[
                ("client_id", self.client_id.as_str()),
                ("omadac_id", self.omadac_id.as_str()),
                ("response_type", "code"),
            ])
            .header("Csrf-Token", &self.csrf_token)
            .header("Cookie", format!("TPOMADA_SESSIONID={}", self.session_id))
            .send_json::<ApiResponse<String>>()
            .await?
            .into_result()
    }
}

// ── OmadaClient ───────────────────────────────────────────────────────────────

/// Async client for the TP-Link Omada Open API.
///
/// Holds a base URL, `omadacId`, and an authenticated HTTP session. The client
/// transparently refreshes its access token before it expires.
#[derive(Debug)]
pub struct OmadaClient {
    base_url: String,
    omadac_id: String,
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
    token: Arc<RwLock<TokenState>>,
}

/// Normalizes a MAC address string to the `AA-BB-CC-DD-EE-FF` format required
/// by AP path parameters. Accepts colons, dashes, or no separators; upper or
/// lower case hex digits.
fn format_mac<'a>(s: &str, buf: &'a mut [u8; 17]) -> &'a str {
    let mut i = 0usize;
    for b in s.bytes() {
        let h = if (b > b'@' && b < b'G') || (b > b'/' && b < b':') {
            b
        } else if b > b'`' && b < b'g' {
            b ^ 0x20 // to uppercase: clear bit 5
        } else {
            continue;
        };
        if i < 17 {
            buf[i] = h;
            i += 1;
            if i < 17 && (i + 1) % 3 == 0 {
                buf[i] = b'-';
                i += 1;
            }
        }
        if i >= 17 {
            break;
        }
    }
    // All bytes written are printable ASCII.
    unsafe { std::str::from_utf8_unchecked(buf) }
}

/// Default page size used by the streaming pagination helpers.
///
/// Callers that need fine-grained control can use the corresponding
/// `*_page` methods directly.
const PAGE_SIZE: u32 = 100;

impl OmadaClient {
    // ── Builder ───────────────────────────────────────────────────────────────

    /// Returns a [`ClientBuilder`] for configuring the client before
    /// authentication (e.g. to disable TLS certificate verification).
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    // ── Constructors ──────────────────────────────────────────────────────────

    /// Authenticates with the **client credentials** grant and returns a ready
    /// client.
    ///
    /// For advanced configuration (e.g. disabling TLS certificate verification)
    /// use [`OmadaClient::builder`] instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API rejects the
    /// credentials.
    pub async fn with_client_credentials(
        base_url: impl Into<String>,
        omadac_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Result<Self> {
        Self::builder()
            .with_client_credentials(base_url, omadac_id, client_id, client_secret)
            .await
    }

    /// Exchanges an authorization code for an access token and returns a ready
    /// client.
    ///
    /// The code is obtained either via the login-page redirect or by calling
    /// [`OmadaClient::login`] followed by [`AuthSession::authorize_code`].
    ///
    /// For advanced configuration (e.g. disabling TLS certificate verification)
    /// use [`OmadaClient::builder`] instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API rejects the code.
    pub async fn with_authorization_code(
        base_url: impl Into<String>,
        omadac_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        code: impl AsRef<str>,
    ) -> Result<Self> {
        Self::builder()
            .with_authorization_code(base_url, omadac_id, client_id, client_secret, code)
            .await
    }

    // ── Authorization-code flow helper ────────────────────────────────────────

    /// Logs in to the Omada controller and returns an [`AuthSession`] that can
    /// be used to obtain an authorization code.
    ///
    /// This is the first step of the authorization-code flow when no redirect
    /// URL is configured on the Open API application.
    ///
    /// For advanced configuration (e.g. disabling TLS certificate verification)
    /// use [`OmadaClient::builder`] instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the login is rejected.
    pub async fn login(
        base_url: impl Into<String>,
        omadac_id: impl Into<String>,
        client_id: impl Into<String>,
        username: &str,
        password: &str,
    ) -> Result<AuthSession> {
        Self::builder()
            .login(base_url, omadac_id, client_id, username, password)
            .await
    }

    // ── Token management ──────────────────────────────────────────────────────

    /// Refreshes the access token using the stored refresh token.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, the refresh token has
    /// expired, or the API returns a non-zero error code.
    pub(crate) async fn refresh_token(&self) -> Result<()> {
        let refresh = self
            .token
            .read()
            .expect("token lock poisoned")
            .refresh_token
            .clone();
        let new_state = Self::fetch_refresh_token(
            &self.http,
            &self.base_url,
            &self.client_id,
            &self.client_secret,
            &refresh,
        )
        .await?;
        *self.token.write().expect("token lock poisoned") = new_state;
        Ok(())
    }

    /// Returns a valid access token, refreshing it automatically when it is
    /// close to expiry.
    ///
    /// # Errors
    ///
    /// Returns an error if the token has expired and the refresh request fails.
    pub(crate) async fn valid_access_token(&self) -> Result<Arc<str>> {
        {
            let state = self.token.read().expect("token lock poisoned");
            if Instant::now() < state.expires_at {
                return Ok(Arc::clone(&state.access_token));
            }
        }
        self.refresh_token().await?;
        Ok(Arc::clone(
            &self.token.read().expect("token lock poisoned").access_token,
        ))
    }

    // ── API methods ───────────────────────────────────────────────────────────

    /// Returns one page of sites visible to the authenticated principal.
    ///
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    /// Pass `search_key` to filter results by name.
    ///
    /// Prefer [`sites`](Self::sites) when you need to iterate over all results
    /// with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn sites_page(
        &self,
        page: u32,
        page_size: u32,
        search_key: Option<&str>,
    ) -> Result<Page<Site>> {
        let token = self.valid_access_token().await?;
        let url = format!("{}/openapi/v1/{}/sites", self.base_url, self.omadac_id);

        let mut builder = self
            .http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)]);

        if let Some(key) = search_key {
            builder = builder.query(&[("searchKey", key)]);
        }

        builder
            .send_json::<ApiResponse<Page<Site>>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`Site`] visible to the
    /// authenticated principal.
    ///
    /// Pass `search_key` to filter results by site name.
    ///
    /// The stream is lazy: network requests are only made as the consumer polls
    /// for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<Site>`; the stream terminates after the first error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use futures_util::TryStreamExt as _;
    /// # async fn example(client: omada_client::OmadaClient) -> omada_client::Result<()> {
    /// let sites: Vec<_> = client.sites(None).try_collect().await?;
    /// # Ok(()) }
    /// ```
    pub fn sites(&self, search_key: Option<String>) -> BoxStream<'_, Result<Site>> {
        let search_key: Option<Arc<str>> = search_key.map(Arc::from);
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<Site>::new()),
            move |(next_page, mut buf)| {
                let key = search_key.clone();
                async move {
                    // Yield buffered items before fetching a new page.
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self.sites_page(page_num, PAGE_SIZE, key.as_deref()).await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<Site> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    // ── WLAN group methods ────────────────────────────────────────────────────

    /// Returns the list of WLAN groups for the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn wlan_groups(&self, site_id: &str) -> Result<Vec<WlanGroup>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<WlanGroup>>>()
            .await?
            .into_result()
    }

    /// Creates a new WLAN group in the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn create_wlan_group(
        &self,
        site_id: &str,
        body: &CreateWlanGroupRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans",
            self.base_url, self.omadac_id
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Deletes an existing WLAN group from the given site.
    ///
    /// The default WLAN group cannot be deleted (API error `-33203`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn delete_wlan_group(&self, site_id: &str, wlan_id: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .delete(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    // ── Scenario methods ──────────────────────────────────────────────────────

    /// Returns the list of available scenario names for use when creating or
    /// modifying a site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn scenarios(&self) -> Result<Vec<String>> {
        let token = self.valid_access_token().await?;
        let url = format!("{}/openapi/v1/{}/scenarios", self.base_url, self.omadac_id);
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<String>>>()
            .await?
            .into_result()
    }

    // ── Profile methods ───────────────────────────────────────────────────────

    /// Returns the list of all group profiles for the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn group_profiles(&self, site_id: &str) -> Result<Vec<GroupProfile>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/groups",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<GroupProfile>>>()
            .await?
            .into_result()
    }

    /// Returns the list of Wi-Fi Calling profiles for the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn wifi_calling_profiles(&self, site_id: &str) -> Result<Vec<WifiCallingProfile>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/wifi-calling",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<WifiCallingProfile>>>()
            .await?
            .into_result()
    }

    /// Creates a new Wi-Fi Calling profile for the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-44900` when a profile with that name already exists,
    /// or `-44901` when the ePDG limit is exceeded).
    pub async fn create_wifi_calling_profile(
        &self,
        site_id: &str,
        body: &CreateWifiCallingProfileRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/wifi-calling",
            self.base_url, self.omadac_id
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Copies an existing Wi-Fi Calling profile, giving the copy a new name.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-44900` when a profile with that name already exists,
    /// or `-44901` when the ePDG limit is exceeded).
    pub async fn copy_wifi_calling_profile(
        &self,
        site_id: &str,
        profile_id: &str,
        body: &CopyWifiCallingProfileRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/wifi-calling/{profile_id}/copy",
            self.base_url, self.omadac_id
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    // ── RADIUS profile methods ────────────────────────────────────────────────

    /// Returns the list of all RADIUS profiles for the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn radius_profiles(&self, site_id: &str) -> Result<Vec<RadiusProfile>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/radius",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<RadiusProfile>>>()
            .await?
            .into_result()
    }

    /// Creates a new RADIUS profile for the given site.
    ///
    /// Returns the ID string of the newly created profile.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-34004` when a profile with that name already exists,
    /// or `-34014` when the RADIUS profile limit is reached).
    pub async fn create_radius_profile(
        &self,
        site_id: &str,
        body: &RadiusProfileRequest,
    ) -> Result<String> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/radius",
            self.base_url, self.omadac_id
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<String>>()
            .await?
            .into_result()
    }

    /// Modifies an existing RADIUS profile.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-34005` when the profile does not exist, or `-34004`
    /// when the new name conflicts with another profile).
    pub async fn update_radius_profile(
        &self,
        site_id: &str,
        radius_profile_id: &str,
        body: &RadiusProfileRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/radius/{radius_profile_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Deletes an existing RADIUS profile from the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-34009` when the profile is used in MAC-Based
    /// Authentication, or `-34013` when it is applied to an SSID).
    pub async fn delete_radius_profile(
        &self,
        site_id: &str,
        radius_profile_id: &str,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/profiles/radius/{radius_profile_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .delete(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    // ── SSID methods ─────────────────────────────────────────────────

    /// Returns one page of SSIDs for the given site WLAN group.
    ///
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    ///
    /// Prefer [`ssids`](Self::ssids) when you need to iterate over all results
    /// with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ssids_page(
        &self,
        site_id: &str,
        wlan_id: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Page<Ssid>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)])
            .send_json::<ApiResponse<Page<Ssid>>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`Ssid`] for the given site WLAN
    /// group.
    ///
    /// The stream is lazy: network requests are only made as the consumer polls
    /// for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<Ssid>`; the stream terminates after the first error.
    #[must_use]
    pub fn ssids(
        &self,
        site_id: impl Into<String>,
        wlan_id: impl Into<String>,
    ) -> BoxStream<'_, Result<Ssid>> {
        let site_id: Arc<str> = Arc::from(site_id.into());
        let wlan_id: Arc<str> = Arc::from(wlan_id.into());
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<Ssid>::new()),
            move |(next_page, mut buf)| {
                let site_id = site_id.clone();
                let wlan_id = wlan_id.clone();
                async move {
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self
                        .ssids_page(&site_id, &wlan_id, page_num, PAGE_SIZE)
                        .await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<Ssid> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    /// Creates a new SSID in the given site WLAN group.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn create_ssid(
        &self,
        site_id: &str,
        wlan_id: &str,
        body: &CreateSsidRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids",
            self.base_url, self.omadac_id
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the full detail of a single SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ssid(&self, site_id: &str, wlan_id: &str, ssid_id: &str) -> Result<SsidDetail> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<SsidDetail>>()
            .await?
            .into_result()
    }

    /// Deletes an existing SSID from the given site WLAN group.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn delete_ssid(&self, site_id: &str, wlan_id: &str, ssid_id: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .delete(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the basic configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_basic_config(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidBasicConfigRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-basic-config",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the WLAN schedule configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_wlan_schedule(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidWlanScheduleRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-wlan-schedule",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the Wi-Fi Calling configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_wifi_calling(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidWifiCallingRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-wifi-calling",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the rate limit configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_rate_limit(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidRateLimitRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-rate-limit",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the 802.11 rate control configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_rate_control(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidRateControlRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-rate-control",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the multicast/broadcast management configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_multicast(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidMulticastRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-multicast-config",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the MAC filter configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_mac_filter(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidMacFilterRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-mac-filter",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the load balance configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_load_balance(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidLoadBalanceRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-load-balance",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the Hotspot 2.0 configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_hotspot_v2(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidHotspotV2Request,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-hotspotv2",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the DHCP Option 82 configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_dhcp_option(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidDhcpOptionRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-dhcp-option",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the band steer configuration of an SSID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ssid_band_steer(
        &self,
        site_id: &str,
        wlan_id: &str,
        ssid_id: &str,
        body: &UpdateSsidBandSteerRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/wireless-network/wlans/{wlan_id}/ssids/{ssid_id}/update-band-steer",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    // ── AP methods ────────────────────────────────────────────────────────────

    /// Returns overview information for the AP with the given MAC address.
    ///
    /// The MAC address is normalised to `AA-BB-CC-DD-EE-FF` format
    /// automatically; colons, dashes, or no separators are all accepted.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-39303` when the AP does not exist).
    pub async fn ap(&self, site_id: &str, ap_mac: &str) -> Result<ApOverview> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApOverview>>()
            .await?
            .into_result()
    }

    /// Returns the wired uplink status of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_wired_uplink(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApWiredUplinkStatus> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/wired-uplink",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApWiredUplinkStatus>>()
            .await?
            .into_result()
    }

    /// Returns the wired downlink devices of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_wired_downlink(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApWiredDownlinkStatus> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/wired-downlink",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApWiredDownlinkStatus>>()
            .await?
            .into_result()
    }

    /// Returns the management VLAN configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_vlan_config(&self, site_id: &str, ap_mac: &str) -> Result<ApVlanConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/vlan",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApVlanConfig>>()
            .await?
            .into_result()
    }

    /// Returns the P2P link speed test results for an AP. Only applicable to
    /// P2P bridge devices.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_speed_test_result(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApSpeedTestResults> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/speed-test-result",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApSpeedTestResults>>()
            .await?
            .into_result()
    }

    /// Returns the SNMP configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_snmp_config(&self, site_id: &str, ap_mac: &str) -> Result<ApSnmpConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/snmp",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApSnmpConfig>>()
            .await?
            .into_result()
    }

    /// Returns the RF scan result for an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_rf_scan_result(&self, site_id: &str, ap_mac: &str) -> Result<ApRfScanResult> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v2/{}/sites/{site_id}/aps/{}/rf-scan-result",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApRfScanResult>>()
            .await?
            .into_result()
    }

    /// Returns current radio channel and traffic detail for an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_radios(&self, site_id: &str, ap_mac: &str) -> Result<ApRadiosDetail> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/radios",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApRadiosDetail>>()
            .await?
            .into_result()
    }

    /// Returns the LAN port list for an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_ports(&self, site_id: &str, ap_mac: &str) -> Result<Vec<ApLanPort>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ports",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<ApLanPort>>>()
            .await?
            .into_result()
    }

    /// Returns a page of current VLAN assignments for an AP's ports.
    ///
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    ///
    /// Prefer [`ap_port_vlans`](Self::ap_port_vlans) when you need to iterate
    /// over all results with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_port_vlans_page(
        &self,
        site_id: &str,
        ap_mac: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Page<ApVlanSummary>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/port-vlans",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)])
            .send_json::<ApiResponse<Page<ApVlanSummary>>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`ApVlanSummary`] for an AP's
    /// ports.
    ///
    /// The stream is lazy: network requests are only made as the consumer polls
    /// for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<ApVlanSummary>`; the stream terminates after the first
    /// error.
    #[must_use]
    pub fn ap_port_vlans(
        &self,
        site_id: impl Into<String>,
        ap_mac: impl Into<String>,
    ) -> BoxStream<'_, Result<ApVlanSummary>> {
        let site_id: Arc<str> = Arc::from(site_id.into());
        let ap_mac: Arc<str> = Arc::from(ap_mac.into());
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<ApVlanSummary>::new()),
            move |(next_page, mut buf)| {
                let site_id = site_id.clone();
                let ap_mac = ap_mac.clone();
                async move {
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self
                        .ap_port_vlans_page(&site_id, &ap_mac, page_num, PAGE_SIZE)
                        .await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<ApVlanSummary> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    /// Returns the bridge pairing window result for an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_paring_window_result(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApBridgeParingWindowResult> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/paring-window-result",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApBridgeParingWindowResult>>()
            .await?
            .into_result()
    }

    /// Returns P2P bridge group information for an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_p2p_info(&self, site_id: &str, ap_mac: &str) -> Result<ApP2pInfo> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/p2pInfo",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApP2pInfo>>()
            .await?
            .into_result()
    }

    /// Returns mesh statistics for an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_mesh_statistics(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApMeshStatistics> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/mesh/statistics",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApMeshStatistics>>()
            .await?
            .into_result()
    }

    /// Returns the LLDP configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_lldp_config(&self, site_id: &str, ap_mac: &str) -> Result<ApLldpConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/lldp",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApLldpConfig>>()
            .await?
            .into_result()
    }

    /// Returns the general configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_general_config(&self, site_id: &str, ap_mac: &str) -> Result<ApGeneralConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/general-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApGeneralConfig>>()
            .await?
            .into_result()
    }

    /// Updates the general configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_general_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApGeneralConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/general-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the IP address settings of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_ip_setting(&self, site_id: &str, ap_mac: &str) -> Result<ApIpSetting> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ip-setting",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApIpSetting>>()
            .await?
            .into_result()
    }

    /// Updates the IP address settings of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_ip_setting(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApIpSetting,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ip-setting",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the IPv6 settings of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_ipv6_setting(&self, site_id: &str, ap_mac: &str) -> Result<ApIpv6Setting> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ipv6-setting",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApIpv6Setting>>()
            .await?
            .into_result()
    }

    /// Updates the IPv6 settings of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_ipv6_setting(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApIpv6Setting,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ipv6-setting",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the radio configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_radio_config(&self, site_id: &str, ap_mac: &str) -> Result<ApRadiosConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/radio-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApRadiosConfig>>()
            .await?
            .into_result()
    }

    /// Updates the radio configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_radio_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApRadiosConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/radio-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the OFDMA configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_ofdma_config(&self, site_id: &str, ap_mac: &str) -> Result<ApOfdmaConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ofdma",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApOfdmaConfig>>()
            .await?
            .into_result()
    }

    /// Updates the OFDMA configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_ofdma_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApOfdmaConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ofdma",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the `QoS` (U-APSD) configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_qos_config(&self, site_id: &str, ap_mac: &str) -> Result<ApQosConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/qos",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApQosConfig>>()
            .await?
            .into_result()
    }

    /// Updates the `QoS` (U-APSD) configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_qos_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApQosConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/qos",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the per-band load-balance configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_load_balance_config(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApLoadBalanceConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/load-balance",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApLoadBalanceConfig>>()
            .await?
            .into_result()
    }

    /// Updates the per-band load-balance configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_load_balance_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApLoadBalanceConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/load-balance",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the trunk setting configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_trunk_setting(&self, site_id: &str, ap_mac: &str) -> Result<ApTrunkSetting> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/trunk-setting",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApTrunkSetting>>()
            .await?
            .into_result()
    }

    /// Updates the trunk setting configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_trunk_setting(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApTrunkSetting,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/trunk-setting",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the service configuration (management VLAN, SNMP, LLDP, `VoIP`
    /// VLAN) of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_service_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApServicesConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/service-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the P2P bridge configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_bridge_config(&self, site_id: &str, ap_mac: &str) -> Result<ApBridgeConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/bridge",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApBridgeConfig>>()
            .await?
            .into_result()
    }

    /// Updates the P2P bridge configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_bridge_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApBridgeConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/bridge",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the antenna gain configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_antenna_gain(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApAntennaGainConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/antenna-gain",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApAntennaGainConfig>>()
            .await?
            .into_result()
    }

    /// Updates the antenna gain configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_antenna_gain(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApAntennaGainConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/antenna-gain",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the uplink port configuration of an AP.
    ///
    /// The API returns a list; multiple entries can appear on devices with more
    /// than one uplink port.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_uplink_config(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<Vec<ApUplinkConfig>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/uplink-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<Vec<ApUplinkConfig>>>()
            .await?
            .into_result()
    }

    /// Updates the uplink port configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_uplink_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApUplinkConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/uplink-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the power saving configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_power_saving_config(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApPowerSavingConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/power-saving",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApPowerSavingConfig>>()
            .await?
            .into_result()
    }

    /// Updates the power saving configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_power_saving_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApPowerSavingConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/power-saving",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the management WLAN (SSID) configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_management_wlan(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApManagementWlan> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/management-wlan",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApManagementWlan>>()
            .await?
            .into_result()
    }

    /// Updates the management WLAN (SSID) configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_management_wlan(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApManagementWlan,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/management-wlan",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the channel limit configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_channel_limit_config(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApChannelLimitConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/channel-limit",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApChannelLimitConfig>>()
            .await?
            .into_result()
    }

    /// Updates the channel limit configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_channel_limit_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApChannelLimitConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/channel-limit",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the channel and radio enable state for a single radio band on
    /// an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_channel_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApChannelConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/channel-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the antenna switch configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_ant_switch_config(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApAntSwitchConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ant-switch",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApAntSwitchConfig>>()
            .await?
            .into_result()
    }

    /// Updates the antenna switch configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_ant_switch_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApAntSwitchConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ant-switch",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns the AFC (Automated Frequency Coordination) configuration of an
    /// AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_afc_config(&self, site_id: &str, ap_mac: &str) -> Result<ApAfcConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/afc-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApAfcConfig>>()
            .await?
            .into_result()
    }

    /// Updates the AFC configuration of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_afc_config(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApAfcConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/afc-config",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Switches the WLAN group of an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_wlan_group(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &UpdateApWlanGroupRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/wlan-group",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates a single LAN port on an AP.
    ///
    /// `port` is the port identifier string (e.g. `"LAN1"`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_ap_port(
        &self,
        site_id: &str,
        ap_mac: &str,
        port: &str,
        body: &UpdateApLanPort,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/ports/{port}",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    // ── AP action methods ─────────────────────────────────────────────────────

    /// Starts an RF scan on an AP.
    ///
    /// Starts an RF scan on an AP.
    ///
    /// Wi-Fi connections will be interrupted for several minutes during the
    /// scan. Pass `body.radio_id_list` to restrict scanning to specific bands
    /// (2.4 GHz = `0`, 5 GHz = `1`, 5 GHz-2 = `2`, 6 GHz = `3`); omit it or
    /// pass `&Default::default()` to scan all bands.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_start_rf_scan(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &RfScanCommand,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v2/{}/sites/{site_id}/aps/{}/start-rf-scan",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Starts a P2P link speed test originating from an AP.
    ///
    /// Only applicable to P2P bridge devices.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_start_speed_test(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApSpeedTestCommand,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/start-speed-test",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Opens the bridge pairing window on an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_start_paring_window(&self, site_id: &str, ap_mac: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/start-paring-window",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Closes the bridge pairing window on an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_stop_paring_window(&self, site_id: &str, ap_mac: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/stop-paring-window",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Starts a spectral (environment) scan on an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_spectral_scan_start(&self, site_id: &str, ap_mac: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/spectral-scan-start",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Stops a spectral (environment) scan on an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_spectral_scan_stop(&self, site_id: &str, ap_mac: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/spectral-scan-stop",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Starts full-channel interference detection on an AP.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_full_channel_detect_start(
        &self,
        site_id: &str,
        ap_mac: &str,
        body: &ApFullChannelDetectRequest,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/full-channel-detect-start",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Updates the AP's location using GPS data from the device's GPS module.
    ///
    /// Only valid for APs that have GPS hardware.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn ap_location_from_gps(
        &self,
        site_id: &str,
        ap_mac: &str,
    ) -> Result<ApLocationConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/{}/location-gps",
            self.base_url,
            self.omadac_id,
            format_mac(ap_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<ApLocationConfig>>()
            .await?
            .into_result()
    }

    // ── Site-level AP methods ─────────────────────────────────────────────────

    /// Moves one or more APs from the given site to another site.
    ///
    /// Pass MAC addresses in `body.ap_macs` to move specific APs; omit the
    /// field to move all APs in the site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn aps_move_site(
        &self,
        site_id: &str,
        body: &ApMoveSiteRequest,
    ) -> Result<MoveSiteResult> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/aps/site-move",
            self.base_url, self.omadac_id
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<MoveSiteResult>>()
            .await?
            .into_result()
    }

    // ── Device methods ────────────────────────────────────────────────────────

    /// Returns one page of devices for the given site.
    ///
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    /// Pass `params` to apply optional search, sort, and filter criteria.
    ///
    /// Prefer [`devices`](Self::devices) when you need to iterate over all
    /// results with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn devices_page(
        &self,
        site_id: &str,
        page: u32,
        page_size: u32,
        params: Option<&DeviceListParams>,
    ) -> Result<Page<DeviceInfo>> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/devices",
            self.base_url, self.omadac_id
        );
        let mut builder = self
            .http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)]);

        if let Some(p) = params {
            if let Some(ref key) = p.search_key {
                builder = builder.query(&[("searchKey", key.as_str())]);
            }
            if let Some(ref v) = p.sort_name {
                builder = builder.query(&[("sorts.name", v.as_str())]);
            }
            if let Some(ref v) = p.sort_status {
                builder = builder.query(&[("sorts.status", v.as_str())]);
            }
            if let Some(ref v) = p.sort_ip {
                builder = builder.query(&[("sorts.ip", v.as_str())]);
            }
            if let Some(ref v) = p.filter_tag {
                builder = builder.query(&[("filters.tag", v.as_str())]);
            }
        }

        builder
            .send_json::<ApiResponse<Page<DeviceInfo>>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`DeviceInfo`] for the given
    /// site.
    ///
    /// Pass `params` to apply optional search, sort, and filter criteria.
    ///
    /// The stream is lazy: network requests are only made as the consumer
    /// polls for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<DeviceInfo>`; the stream terminates after the first
    /// error.
    #[must_use]
    pub fn devices(
        &self,
        site_id: impl Into<String>,
        params: Option<DeviceListParams>,
    ) -> BoxStream<'_, Result<DeviceInfo>> {
        let site_id: Arc<str> = Arc::from(site_id.into());
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<DeviceInfo>::new()),
            move |(next_page, mut buf)| {
                let params = params.clone();
                let site_id = site_id.clone();
                async move {
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self
                        .devices_page(&site_id, page_num, PAGE_SIZE, params.as_ref())
                        .await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<DeviceInfo> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    // ── LAN network methods ───────────────────────────────────────────────────

    /// Returns one page of LAN networks for the given site.
    ///
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    ///
    /// The returned [`LanNetworkPage`] includes site-level capability metadata
    /// (e.g. `support_multi_vlan`, `dhcp_range_pool_size`) alongside the
    /// standard pagination fields.
    ///
    /// Prefer [`lan_networks`](Self::lan_networks) when you need to iterate
    /// over all results with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn lan_networks_page(
        &self,
        site_id: &str,
        page: u32,
        page_size: u32,
    ) -> Result<LanNetworkPage> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v3/{}/sites/{site_id}/lan-networks",
            self.base_url, self.omadac_id
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)])
            .send_json::<ApiResponse<LanNetworkPage>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`LanNetwork`] for the given
    /// site.
    ///
    /// The stream is lazy: network requests are only made as the consumer
    /// polls for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<LanNetwork>`; the stream terminates after the first
    /// error.
    #[must_use]
    pub fn lan_networks(&self, site_id: impl Into<String>) -> BoxStream<'_, Result<LanNetwork>> {
        let site_id: Arc<str> = Arc::from(site_id.into());
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<LanNetwork>::new()),
            move |(next_page, mut buf)| {
                let site_id = site_id.clone();
                async move {
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self
                        .lan_networks_page(&site_id, page_num, PAGE_SIZE)
                        .await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<LanNetwork> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    // ── LAN profile methods ───────────────────────────────────────────────────

    /// Returns one page of LAN profiles for the given site.
    ///
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    ///
    /// Prefer [`lan_profiles`](Self::lan_profiles) when you need to iterate
    /// over all results with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn lan_profiles_page(
        &self,
        site_id: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Page<LanProfile>> {
        let token = self.valid_access_token().await?;
        self.http
            .get(format!(
                "{}/openapi/v1/{}/sites/{site_id}/lan-profiles",
                self.base_url, self.omadac_id
            ))
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)])
            .send_json::<ApiResponse<Page<LanProfile>>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`LanProfile`] for the given
    /// site.
    ///
    /// The stream is lazy: network requests are only made as the consumer polls
    /// for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<LanProfile>`; the stream terminates after the first
    /// error.
    #[must_use]
    pub fn lan_profiles(&self, site_id: impl Into<String>) -> BoxStream<'_, Result<LanProfile>> {
        let site_id: Arc<str> = Arc::from(site_id.into());
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<LanProfile>::new()),
            move |(next_page, mut buf)| {
                let site_id = site_id.clone();
                async move {
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self
                        .lan_profiles_page(&site_id, page_num, PAGE_SIZE)
                        .await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<LanProfile> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    /// Creates a new LAN profile in the given site.
    ///
    /// Returns the ID of the newly created profile.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn create_lan_profile(
        &self,
        site_id: &str,
        body: &LanProfileConfig,
    ) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct IdResult {
            id: Option<String>,
        }
        let token = self.valid_access_token().await?;
        let result = self
            .http
            .post(format!(
                "{}/openapi/v1/{}/sites/{site_id}/lan-profiles",
                self.base_url, self.omadac_id
            ))
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<IdResult>>()
            .await?
            .into_result()?;
        Ok(result.id.unwrap_or_default())
    }

    /// Deletes an existing LAN profile from the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code. Error `-33560` indicates the profile is in use by an Agile
    /// Series Switch and cannot be deleted.
    pub async fn delete_lan_profile(&self, site_id: &str, profile_id: &str) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/lan-profiles/{profile_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .delete(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Modifies an existing LAN profile in the given site.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_lan_profile(
        &self,
        site_id: &str,
        profile_id: &str,
        body: &LanProfileConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/lan-profiles/{profile_id}",
            self.base_url, self.omadac_id
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    // ── Switch methods ────────────────────────────────────────────────────────

    /// Returns overview information for a switch.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-39050` when the device does not exist).
    pub async fn switch(&self, site_id: &str, switch_mac: &str) -> Result<SwitchInfo> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<SwitchInfo>>()
            .await?
            .into_result()
    }

    /// Returns the general configuration of a switch.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-39700` when the switch does not exist).
    pub async fn switch_general_config(
        &self,
        site_id: &str,
        switch_mac: &str,
    ) -> Result<SwitchGeneralConfig> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}/general-config",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .send_json::<ApiResponse<SwitchGeneralConfig>>()
            .await?
            .into_result()
    }

    /// Updates the general configuration of a switch.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-39700` when the switch does not exist).
    pub async fn update_switch_general_config(
        &self,
        site_id: &str,
        switch_mac: &str,
        body: &SwitchGeneralConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}/general-config",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Modifies the configuration of a single switch port.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    /// `port` is the port number as a string (e.g. `"1"`).
    ///
    /// Only the fields set (non-`None`) in `body` will be sent to the API.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code.
    pub async fn update_switch_port(
        &self,
        site_id: &str,
        switch_mac: &str,
        port: &str,
        body: &SwitchPortConfig,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}/ports/{port}",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .patch(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Sets the LAN profile bound to a switch port.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    /// `port` is the port number as a string (e.g. `"1"`).
    /// `profile_id` is the profile ID to bind.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-33507` when the profile does not exist, `-39701` when
    /// the port does not exist).
    pub async fn set_switch_port_profile(
        &self,
        site_id: &str,
        switch_mac: &str,
        port: &str,
        profile_id: &str,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Body<'a> {
            #[serde(rename = "profileId")]
            profile_id: &'a str,
        }
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}/ports/{port}/profile",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .put(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(&Body { profile_id })?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    /// Returns a page of VLAN interface configurations for a switch.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    /// `page` is 1-based. `page_size` must be in the range 1–1000.
    ///
    /// Prefer [`switch_networks`](Self::switch_networks) when you need to
    /// iterate over all results with automatic pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-39700` when the switch does not exist).
    pub async fn switch_networks_page(
        &self,
        site_id: &str,
        switch_mac: &str,
        page: u32,
        page_size: u32,
    ) -> Result<SwitchNetworksPage> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}/networks",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .get(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .query(&[("page", page), ("pageSize", page_size)])
            .send_json::<ApiResponse<SwitchNetworksPage>>()
            .await?
            .into_result()
    }

    /// Returns a [`Stream`] that yields every [`SwitchNetwork`] for the given
    /// switch.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    ///
    /// The stream is lazy: network requests are only made as the consumer
    /// polls for more items. Dropping the stream early stops further requests.
    ///
    /// Items are `Result<SwitchNetwork>`; the stream terminates after the
    /// first error.
    #[must_use]
    pub fn switch_networks(
        &self,
        site_id: impl Into<String>,
        switch_mac: impl Into<String>,
    ) -> BoxStream<'_, Result<SwitchNetwork>> {
        let site_id: Arc<str> = Arc::from(site_id.into());
        let switch_mac: Arc<str> = Arc::from(switch_mac.into());
        futures_util::stream::try_unfold(
            (Some(1u32), VecDeque::<SwitchNetwork>::new()),
            move |(next_page, mut buf)| {
                let site_id = site_id.clone();
                let switch_mac = switch_mac.clone();
                async move {
                    if let Some(item) = buf.pop_front() {
                        return Ok(Some((item, (next_page, buf))));
                    }
                    let Some(page_num) = next_page else {
                        return Ok(None);
                    };
                    let page = self
                        .switch_networks_page(&site_id, &switch_mac, page_num, PAGE_SIZE)
                        .await?;
                    let fetched = i64::from(page_num) * i64::from(PAGE_SIZE);
                    let next = if fetched < page.total_rows {
                        Some(page_num + 1)
                    } else {
                        None
                    };
                    let mut new_buf: VecDeque<SwitchNetwork> = page.data.into();
                    match new_buf.pop_front() {
                        Some(item) => Ok(Some((item, (next, new_buf)))),
                        None => Ok(None),
                    }
                }
            },
        )
        .boxed()
    }

    /// Modifies the VLAN interface configuration of a switch network.
    ///
    /// `switch_mac` is the MAC address of the switch in any common format
    /// (e.g. `AA-BB-CC-DD-EE-FF`, `aa:bb:cc:dd:ee:ff`).
    /// `network_id` is the network ID returned by [`OmadaClient::switch_networks_page`].
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the API returns a non-zero
    /// error code (e.g. `-39700` when the switch does not exist, or `-33529`
    /// when disabling the management VLAN is attempted).
    pub async fn update_switch_network(
        &self,
        site_id: &str,
        switch_mac: &str,
        network_id: &str,
        body: &SwitchNetwork,
    ) -> Result<()> {
        let token = self.valid_access_token().await?;
        let url = format!(
            "{}/openapi/v1/{}/sites/{site_id}/switches/{}/networks/{network_id}",
            self.base_url,
            self.omadac_id,
            format_mac(switch_mac, &mut [0u8; 17])
        );
        self.http
            .post(&url)
            .header("Authorization", format!("AccessToken={token}"))
            .body_json(body)?
            .send_json::<ApiResponse<json::Value>>()
            .await?
            .check()
    }

    async fn fetch_client_credentials_token(
        http: &reqwest::Client,
        base_url: &str,
        omadac_id: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<TokenState> {
        let url = format!("{base_url}/openapi/authorize/token");
        http.post(&url)
            .query(&[("grant_type", "client_credentials")])
            .body_json(&ClientCredentialsBody {
                omadac_id,
                client_id,
                client_secret,
            })?
            .send_json::<ApiResponse<TokenResult>>()
            .await?
            .into_result()
            .map(token_state_from)
    }

    async fn fetch_auth_code_token(
        http: &reqwest::Client,
        base_url: &str,
        client_id: &str,
        client_secret: &str,
        code: &str,
    ) -> Result<TokenState> {
        let url = format!("{base_url}/openapi/authorize/token");
        http.post(&url)
            .query(&[("grant_type", "authorization_code"), ("code", code)])
            .body_json(&ClientAuthBody {
                client_id,
                client_secret,
            })?
            .send_json::<ApiResponse<TokenResult>>()
            .await?
            .into_result()
            .map(token_state_from)
    }

    async fn fetch_refresh_token(
        http: &reqwest::Client,
        base_url: &str,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<TokenState> {
        let url = format!("{base_url}/openapi/authorize/token");
        http.post(&url)
            .query(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .body_json(&ClientAuthBody {
                client_id,
                client_secret,
            })?
            .send_json::<ApiResponse<TokenResult>>()
            .await?
            .into_result()
            .map(token_state_from)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{body_partial_json, header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    const CLIENT_ID: &str = "29f2fdbeb5a84d50b9b1cdd08cd1a3ff";
    const CLIENT_SECRET: &str = "cf6b13af0dd045628c9f05f088eb5493";
    const OMADAC_ID: &str = "de382a0e78f4deb681f3128c3e75dbd1";

    fn token_body(access: &str, refresh: &str) -> serde_json::Value {
        serde_json::json!({
            "errorCode": 0,
            "msg": "Open API Get Access Token successfully.",
            "result": {
                "accessToken": access,
                "tokenType": "bearer",
                "expiresIn": 7200,
                "refreshToken": refresh
            }
        })
    }

    // ── 2.2.1 Login ───────────────────────────────────────────────────────────

    /// POST /openapi/authorize/login
    ///   Query: client_id, omadac_id
    ///   Body:  { username, password }
    ///   Returns: { csrfToken, sessionId }
    #[tokio::test]
    async fn login_posts_credentials_and_returns_session() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/login"))
            .and(query_param("client_id", CLIENT_ID))
            .and(query_param("omadac_id", OMADAC_ID))
            .and(body_partial_json(serde_json::json!({
                "username": "admin",
                "password": "tplink123"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Open API Log in successfully.",
                "result": {
                    "csrfToken": "51cb83c7d00a4e85b3dd2d6174a614d9",
                    "sessionId": "2fc0ca155ab94957a9a9e6a3b00662ea"
                }
            })))
            .mount(&server)
            .await;

        let session = OmadaClient::login(server.uri(), OMADAC_ID, CLIENT_ID, "admin", "tplink123")
            .await
            .unwrap();

        assert_eq!(session.csrf_token(), "51cb83c7d00a4e85b3dd2d6174a614d9");
        assert_eq!(session.session_id(), "2fc0ca155ab94957a9a9e6a3b00662ea");
    }

    // ── 2.2.1 Authorization code ──────────────────────────────────────────────

    /// Full authorization-code flow (no redirect URL):
    ///   1. POST /openapi/authorize/login   → csrfToken + sessionId
    ///   2. POST /openapi/authorize/code    → authorization code
    ///      Headers: Csrf-Token, Cookie: TPOMADA_SESSIONID=...
    ///      Query:   client_id, omadac_id, response_type=code
    ///   3. POST /openapi/authorize/token?grant_type=authorization_code&code=...
    ///      Body:  { client_id, client_secret }
    ///      Returns: accessToken + refreshToken
    #[tokio::test]
    async fn auth_code_flow_completes_full_handshake() {
        let server = MockServer::start().await;

        // Step 1 — login
        Mock::given(method("POST"))
            .and(path("/openapi/authorize/login"))
            .and(query_param("client_id", CLIENT_ID))
            .and(query_param("omadac_id", OMADAC_ID))
            .and(body_partial_json(serde_json::json!({
                "username": "admin",
                "password": "tplink123"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Open API Log in successfully.",
                "result": {
                    "csrfToken": "ae6b935c92cf4b1b9f3eb852e20ed2b8",
                    "sessionId": "9cb86bf3a99e48a59e4f3bb464a3c443"
                }
            })))
            .mount(&server)
            .await;

        // Step 2 — obtain authorization code
        Mock::given(method("POST"))
            .and(path("/openapi/authorize/code"))
            .and(query_param("client_id", CLIENT_ID))
            .and(query_param("omadac_id", OMADAC_ID))
            .and(query_param("response_type", "code"))
            .and(header("Csrf-Token", "ae6b935c92cf4b1b9f3eb852e20ed2b8"))
            .and(header(
                "Cookie",
                "TPOMADA_SESSIONID=9cb86bf3a99e48a59e4f3bb464a3c443",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Open API Authorize successfully.",
                "result": "OC-9iyxaKVOVMBpYhQ4NryaYBjghj3dTY32"
            })))
            .mount(&server)
            .await;

        // Step 3 — exchange code for access token
        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "authorization_code"))
            .and(query_param("code", "OC-9iyxaKVOVMBpYhQ4NryaYBjghj3dTY32"))
            .and(body_partial_json(serde_json::json!({
                "client_id": CLIENT_ID,
                "client_secret": CLIENT_SECRET
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body(
                "AT-bllLYOOYASck11SBSDmmHs85lCrkN6Gi",
                "RT-HqvaDuSxEqayM75U2ukTRnBl6f6fiRAc",
            )))
            .mount(&server)
            .await;

        let session = OmadaClient::login(server.uri(), OMADAC_ID, CLIENT_ID, "admin", "tplink123")
            .await
            .unwrap();

        let code = session.authorize_code().await.unwrap();
        assert_eq!(code, "OC-9iyxaKVOVMBpYhQ4NryaYBjghj3dTY32");

        let client = OmadaClient::with_authorization_code(
            server.uri(),
            OMADAC_ID,
            CLIENT_ID,
            CLIENT_SECRET,
            &code,
        )
        .await
        .unwrap();

        assert_eq!(
            client.token.read().unwrap().access_token.as_ref(),
            "AT-bllLYOOYASck11SBSDmmHs85lCrkN6Gi"
        );
    }

    // ── 2.3.1 Client credentials ──────────────────────────────────────────────

    /// POST /openapi/authorize/token?grant_type=client_credentials
    ///   Body: { omadacId, client_id, client_secret }
    ///   Returns: accessToken + refreshToken
    #[tokio::test]
    async fn client_credentials_fetches_token() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .and(body_partial_json(serde_json::json!({
                "omadacId": OMADAC_ID,
                "client_id": CLIENT_ID,
                "client_secret": CLIENT_SECRET
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body(
                "AT-bjaJkIMIiekZY6NBufoQO4hdmJTswlwU",
                "RT-3ZjJgcORJSh76UCh7pj0rs5VRISIpagV",
            )))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        assert_eq!(
            client.token.read().unwrap().access_token.as_ref(),
            "AT-bjaJkIMIiekZY6NBufoQO4hdmJTswlwU"
        );
    }

    // ── 2.2.4 / 2.3.3 Refresh token ──────────────────────────────────────────

    /// POST /openapi/authorize/token?grant_type=refresh_token&refresh_token=...
    ///   Body: { client_id, client_secret }
    ///   Returns: new accessToken + new refreshToken
    #[tokio::test]
    async fn refresh_token_uses_refresh_grant_and_updates_state() {
        let server = MockServer::start().await;

        // Initial client-credentials fetch
        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body(
                "AT-initial",
                "RT-AhzwqCenDCZ84qpBHnZhYs3j2RGw9q8E",
            )))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Refresh call must use the exact refresh token and correct body
        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "refresh_token"))
            .and(query_param(
                "refresh_token",
                "RT-AhzwqCenDCZ84qpBHnZhYs3j2RGw9q8E",
            ))
            .and(body_partial_json(serde_json::json!({
                "client_id": CLIENT_ID,
                "client_secret": CLIENT_SECRET
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body(
                "AT-w9veJNQlaK8dH08qEQZCTas6y70IRAii",
                "RT-AhzwqCenDCZ84qpBHnZhYs3j2RGw9q8E",
            )))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        assert_eq!(
            client.token.read().unwrap().access_token.as_ref(),
            "AT-initial"
        );

        client.refresh_token().await.unwrap();

        assert_eq!(
            client.token.read().unwrap().access_token.as_ref(),
            "AT-w9veJNQlaK8dH08qEQZCTas6y70IRAii"
        );
    }

    // ── sites() ───────────────────────────────────────────────────────────────

    /// GET /openapi/v1/{omadacId}/sites?page=1&pageSize=10
    ///   Authorization: AccessToken={token}
    ///   Returns: Page<Site>
    #[tokio::test]
    async fn sites_returns_paged_site_list() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sites-test", "RT-sites-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/sites")))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "10"))
            .and(header("Authorization", "AccessToken=AT-sites-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 2,
                    "currentPage": 1,
                    "currentSize": 10,
                    "data": [
                        {
                            "siteId": "6017de13f0b9840878c2e29b",
                            "name": "Main Office",
                            "region": "United States",
                            "timeZone": "UTC-8",
                            "scenario": "Office",
                            "primary": true
                        },
                        {
                            "siteId": "6017de13f0b9840878c2e29c",
                            "name": "Branch Office",
                            "region": "United States",
                            "timeZone": "UTC-8",
                            "scenario": "Office",
                            "primary": false
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client.sites_page(1, 10, None).await.unwrap();
        assert_eq!(page.total_rows, 2);
        assert_eq!(page.current_page, 1);
        assert_eq!(page.data.len(), 2);
        assert_eq!(page.data[0].site_id, "6017de13f0b9840878c2e29b");
        assert_eq!(page.data[0].name, "Main Office");
        assert_eq!(page.data[1].site_id, "6017de13f0b9840878c2e29c");
    }

    /// searchKey query param is forwarded when provided
    #[tokio::test]
    async fn sites_forwards_search_key() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-search-test", "RT-search-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/sites")))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "100"))
            .and(query_param("searchKey", "Office"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 1,
                    "currentPage": 1,
                    "currentSize": 100,
                    "data": [
                        {
                            "siteId": "6017de13f0b9840878c2e29b",
                            "name": "Main Office"
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client.sites_page(1, 100, Some("Office")).await.unwrap();
        assert_eq!(page.total_rows, 1);
        assert_eq!(page.data[0].name, "Main Office");
    }

    /// A non-zero errorCode from the sites endpoint surfaces as Error::Api.
    #[tokio::test]
    async fn sites_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-err-test", "RT-err-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/sites")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -1000,
                "msg": "Internal error",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.sites_page(1, 10, None).await.unwrap_err();
        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -1000),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    /// sites() auto-paginates: fetches multiple pages and yields all items in order.
    #[tokio::test]
    async fn sites_stream_paginates_automatically() {
        use futures_util::TryStreamExt as _;

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-stream-test", "RT-stream-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Page 1 — 2 items, totalRows=102 tells the client another page exists.
        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/sites")))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 102,
                    "currentPage": 1,
                    "currentSize": 100,
                    "data": [
                        { "siteId": "site-1", "name": "Site One" },
                        { "siteId": "site-2", "name": "Site Two" }
                    ]
                }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Page 2 — 1 item, totalRows=102, fetched=200 >= 102 so no page 3.
        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/sites")))
            .and(query_param("page", "2"))
            .and(query_param("pageSize", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 102,
                    "currentPage": 2,
                    "currentSize": 100,
                    "data": [
                        { "siteId": "site-3", "name": "Site Three" }
                    ]
                }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let sites: Vec<Site> = client.sites(None).try_collect().await.unwrap();

        assert_eq!(sites.len(), 3);
        assert_eq!(sites[0].site_id, "site-1");
        assert_eq!(sites[1].site_id, "site-2");
        assert_eq!(sites[2].site_id, "site-3");
    }

    /// An API error mid-stream is surfaced as an Err item and ends the stream.
    #[tokio::test]
    async fn sites_stream_propagates_api_error() {
        use futures_util::StreamExt as _;

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-err-stream", "RT-err-stream")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/sites")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -1000,
                "msg": "Internal error",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let first = client.sites(None).next().await.unwrap();
        match first {
            Err(Error::Api { error_code, .. }) => assert_eq!(error_code, -1000),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── Error handling ────────────────────────────────────────────────────────

    /// A non-zero errorCode in the response body must surface as Error::Api.
    /// Error -44106: "The client id or client secret is invalid"
    #[tokio::test]
    async fn invalid_credentials_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -44106,
                "msg": "The client id or client secret is invalid",
                "result": null
            })))
            .mount(&server)
            .await;

        let err =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, "bad_id", "bad_secret")
                .await
                .unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -44106),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── wlan_groups() / create_wlan_group() / delete_wlan_group() ─────────────

    const SITE_ID: &str = "6017de13f0b9840878c2e29b";

    /// GET /openapi/v1/{omadacId}/sites/{siteId}/wireless-network/wlans
    ///   Returns: Vec<WlanGroup>
    #[tokio::test]
    async fn wlan_groups_returns_list() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-wlans-test", "RT-wlans-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans"
            )))
            .and(header("Authorization", "AccessToken=AT-wlans-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": [
                    {
                        "wlanId": "wlan-001",
                        "name": "Default",
                        "primary": true,
                        "clone": false,
                        "site": SITE_ID,
                        "resource": 0
                    },
                    {
                        "wlanId": "wlan-002",
                        "name": "Guest",
                        "primary": false,
                        "clone": false,
                        "site": SITE_ID,
                        "resource": 0
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let groups = client.wlan_groups(SITE_ID).await.unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].wlan_id, "wlan-001");
        assert_eq!(groups[0].name, "Default");
        assert_eq!(groups[0].primary, Some(true));
        assert_eq!(groups[1].wlan_id, "wlan-002");
        assert_eq!(groups[1].name, "Guest");
    }

    /// POST /openapi/v1/{omadacId}/sites/{siteId}/wireless-network/wlans
    ///   Body: { name, clone }
    ///   Returns: ()
    #[tokio::test]
    async fn create_wlan_group_posts_body_and_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-create-wlan", "RT-create-wlan")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans"
            )))
            .and(header("Authorization", "AccessToken=AT-create-wlan"))
            .and(body_partial_json(serde_json::json!({
                "name": "Corp",
                "clone": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Create WLAN group successfully."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .create_wlan_group(
                SITE_ID,
                &CreateWlanGroupRequest {
                    name: "Corp".to_owned(),
                    clone: false,
                    cloned_wlan_id: None,
                },
            )
            .await
            .unwrap();
    }

    /// DELETE /openapi/v1/{omadacId}/sites/{siteId}/wireless-network/wlans/{wlanId}
    ///   Returns: ()
    #[tokio::test]
    async fn delete_wlan_group_sends_delete_and_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-delete-wlan", "RT-delete-wlan")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/wlan-002"
            )))
            .and(header("Authorization", "AccessToken=AT-delete-wlan"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Delete WLAN group successfully."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client.delete_wlan_group(SITE_ID, "wlan-002").await.unwrap();
    }

    /// delete_wlan_group returns Error::Api(-33203) when deleting the default group.
    #[tokio::test]
    async fn delete_wlan_group_default_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-delete-wlan-err", "RT-delete-wlan-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/wlan-001"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33203,
                "msg": "The default WLAN group cannot be deleted."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client
            .delete_wlan_group(SITE_ID, "wlan-001")
            .await
            .unwrap_err();
        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33203),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── scenarios() ───────────────────────────────────────────────────────────

    /// GET /openapi/v1/{omadacId}/scenarios
    ///   Authorization: AccessToken={token}
    ///   Returns: Vec<String>
    #[tokio::test]
    async fn scenarios_returns_list() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-scenarios-test", "RT-scenarios-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("/openapi/v1/{OMADAC_ID}/scenarios")))
            .and(header("Authorization", "AccessToken=AT-scenarios-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": ["Office", "Hotel", "School", "Hospital", "Others"]
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let list = client.scenarios().await.unwrap();
        assert_eq!(
            list,
            vec!["Office", "Hotel", "School", "Hospital", "Others"]
        );
    }

    // ── group_profiles() ──────────────────────────────────────────────────────

    /// GET /openapi/v1/{omadacId}/sites/{siteId}/profiles/groups
    ///   Authorization: AccessToken={token}
    ///   Returns: Vec<GroupProfile>
    #[tokio::test]
    async fn group_profiles_returns_list() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-groups-test", "RT-groups-test")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/groups"
            )))
            .and(header("Authorization", "AccessToken=AT-groups-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": [
                    {
                        "groupId": "group-001",
                        "name": "Internal Hosts",
                        "type": 0,
                        "count": 2,
                        "buildIn": false,
                        "ipList": [
                            { "ip": "192.168.1.0", "mask": 24 },
                            { "ip": "10.0.0.0", "mask": 8 }
                        ]
                    },
                    {
                        "groupId": "group-002",
                        "name": "Office Devices",
                        "type": 2,
                        "count": 1,
                        "buildIn": false,
                        "macAddressList": [
                            { "ruleId": 1, "name": "Printer", "macAddress": "AA:BB:CC:DD:EE:FF" }
                        ]
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let groups = client.group_profiles(SITE_ID).await.unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].group_id, "group-001");
        assert_eq!(groups[0].name, "Internal Hosts");
        assert_eq!(groups[0].group_type, 0);
        let ip_list = groups[0].ip_list.as_ref().unwrap();
        assert_eq!(ip_list.len(), 2);
        assert_eq!(ip_list[0].ip, "192.168.1.0");
        assert_eq!(ip_list[0].mask, 24);
        assert_eq!(groups[1].group_id, "group-002");
        assert_eq!(groups[1].group_type, 2);
        let macs = groups[1].mac_address_list.as_ref().unwrap();
        assert_eq!(macs[0].mac_address, "AA:BB:CC:DD:EE:FF");
    }

    /// A non-zero errorCode from group_profiles surfaces as Error::Api.
    #[tokio::test]
    async fn group_profiles_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-groups-err", "RT-groups-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/groups"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33004,
                "msg": "Operation failed because other operations are being performed on this site.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.group_profiles(SITE_ID).await.unwrap_err();
        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33004),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── radius_profiles() / create_radius_profile() / update_radius_profile()
    //    / delete_radius_profile() ────────────────────────────────────────────

    const RADIUS_PROFILE_ID: &str = "radius-profile-001";

    /// GET /openapi/v1/{omadacId}/sites/{siteId}/profiles/radius
    ///   Returns: Vec<RadiusProfile>
    #[tokio::test]
    async fn radius_profiles_returns_list() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-radius-list", "RT-radius-list")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/radius"
            )))
            .and(header("Authorization", "AccessToken=AT-radius-list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": [
                    {
                        "radiusProfileId": "radius-profile-001",
                        "name": "Corp RADIUS",
                        "authServer": [
                            {
                                "radiusServerIp": "192.168.1.100",
                                "radiusPort": 1812,
                                "radiusPwd": "secret123"
                            }
                        ],
                        "radiusAccountingEnable": false,
                        "wirelessVlanAssignment": true,
                        "builtInServer": false
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let profiles = client.radius_profiles(SITE_ID).await.unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(
            profiles[0].radius_profile_id.as_deref(),
            Some("radius-profile-001")
        );
        assert_eq!(profiles[0].name.as_deref(), Some("Corp RADIUS"));
        let auth = profiles[0].auth_server.as_ref().unwrap();
        assert_eq!(auth[0].radius_server_ip, "192.168.1.100");
        assert_eq!(auth[0].radius_port, 1812);
        assert_eq!(profiles[0].radius_accounting_enable, Some(false));
        assert_eq!(profiles[0].wireless_vlan_assignment, Some(true));
    }

    /// A non-zero errorCode from radius_profiles surfaces as Error::Api.
    #[tokio::test]
    async fn radius_profiles_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-radius-err", "RT-radius-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/radius"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33004,
                "msg": "Operation failed because other operations are being performed on this site.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.radius_profiles(SITE_ID).await.unwrap_err();
        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33004),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    /// POST /openapi/v1/{omadacId}/sites/{siteId}/profiles/radius
    ///   Returns: String (new profile ID)
    #[tokio::test]
    async fn create_radius_profile_posts_body_and_returns_id() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-radius-create", "RT-radius-create")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/radius"
            )))
            .and(header("Authorization", "AccessToken=AT-radius-create"))
            .and(body_partial_json(serde_json::json!({
                "name": "Guest RADIUS",
                "radiusAccountingEnable": false,
                "wirelessVlanAssignment": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": "radius-profile-new"
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let id = client
            .create_radius_profile(
                SITE_ID,
                &RadiusProfileRequest {
                    name: "Guest RADIUS".to_owned(),
                    auth_server: vec![RadiusAuthServer {
                        radius_server_ip: "10.0.0.1".to_owned(),
                        radius_port: 1812,
                        radius_pwd: "topsecret".to_owned(),
                        rad_sec_enable: None,
                        ca_cert: None,
                        client_cert: None,
                    }],
                    radius_accounting_enable: false,
                    interim_update_enable: None,
                    interim_update_interval: None,
                    acct_server: None,
                    wireless_vlan_assignment: false,
                    coa_enable: None,
                    coa_password: None,
                    require_message_authenticator: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(id, "radius-profile-new");
    }

    /// PATCH /openapi/v1/{omadacId}/sites/{siteId}/profiles/radius/{radiusProfileId}
    #[tokio::test]
    async fn update_radius_profile_sends_patch_and_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-radius-update", "RT-radius-update")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/radius/{RADIUS_PROFILE_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-radius-update"))
            .and(body_partial_json(serde_json::json!({
                "name": "Corp RADIUS Updated",
                "radiusAccountingEnable": true,
                "wirelessVlanAssignment": true
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_radius_profile(
                SITE_ID,
                RADIUS_PROFILE_ID,
                &RadiusProfileRequest {
                    name: "Corp RADIUS Updated".to_owned(),
                    auth_server: vec![RadiusAuthServer {
                        radius_server_ip: "192.168.1.100".to_owned(),
                        radius_port: 1812,
                        radius_pwd: "newsecret".to_owned(),
                        rad_sec_enable: None,
                        ca_cert: None,
                        client_cert: None,
                    }],
                    radius_accounting_enable: true,
                    interim_update_enable: Some(true),
                    interim_update_interval: Some(300),
                    acct_server: Some(vec![RadiusAcctServer {
                        accounting_server_ip: "192.168.1.101".to_owned(),
                        accounting_server_port: 1813,
                        accounting_server_pwd: "acctpwd".to_owned(),
                        rad_sec_enable: None,
                        ca_cert: None,
                        client_cert: None,
                    }]),
                    wireless_vlan_assignment: true,
                    coa_enable: None,
                    coa_password: None,
                    require_message_authenticator: None,
                },
            )
            .await
            .unwrap();
    }

    /// DELETE /openapi/v1/{omadacId}/sites/{siteId}/profiles/radius/{radiusProfileId}
    #[tokio::test]
    async fn delete_radius_profile_sends_delete_and_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-radius-delete", "RT-radius-delete")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/profiles/radius/{RADIUS_PROFILE_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-radius-delete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .delete_radius_profile(SITE_ID, RADIUS_PROFILE_ID)
            .await
            .unwrap();
    }

    // ── SSID methods ──────────────────────────────────────────────────────────

    const WLAN_ID: &str = "wlan-abc";
    const SSID_ID: &str = "ssid-xyz";

    /// GET .../ssids?page=1&pageSize=10  →  Page<Ssid>
    #[tokio::test]
    async fn ssids_page_returns_paged_list() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-ssids-list", "RT-ssids-list")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids"
            )))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "10"))
            .and(header("Authorization", "AccessToken=AT-ssids-list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 2,
                    "currentPage": 1,
                    "currentSize": 10,
                    "data": [
                        { "ssidId": "ssid-001", "name": "Corp WiFi", "band": 3, "security": 3, "broadcast": true, "vlanEnable": false },
                        { "ssidId": "ssid-002", "name": "Guest", "band": 3, "security": 0, "broadcast": true, "vlanEnable": false }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client.ssids_page(SITE_ID, WLAN_ID, 1, 10).await.unwrap();
        assert_eq!(page.total_rows, 2);
        assert_eq!(page.data.len(), 2);
        assert_eq!(page.data[0].ssid_id, "ssid-001");
        assert_eq!(page.data[0].name, "Corp WiFi");
        assert_eq!(page.data[1].ssid_id, "ssid-002");
    }

    /// POST .../ssids  with required fields succeeds
    #[tokio::test]
    async fn create_ssid_posts_body_and_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-create-ssid", "RT-create-ssid")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids"
            )))
            .and(header("Authorization", "AccessToken=AT-create-ssid"))
            .and(body_partial_json(serde_json::json!({
                "name": "NewSSID",
                "deviceType": 1,
                "band": 3,
                "security": 0,
                "broadcast": true,
                "vlanEnable": false,
                "mloEnable": false,
                "pmfMode": 2,
                "enable11r": false,
                "hidePwd": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Create SSID successfully."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .create_ssid(
                SITE_ID,
                WLAN_ID,
                &CreateSsidRequest {
                    name: "NewSSID".to_owned(),
                    device_type: 1,
                    band: 3,
                    guest_net_enable: false,
                    security: 0,
                    broadcast: true,
                    vlan_enable: false,
                    vlan_id: None,
                    psk_setting: None,
                    ent_setting: None,
                    ppsk_setting: None,
                    mlo_enable: false,
                    pmf_mode: 2,
                    enable11r: false,
                    hide_pwd: false,
                    gre_enable: None,
                    vlan_setting: None,
                    prohibit_wifi_share: None,
                    wifi_calling_enable: None,
                    wifi_calling_id: None,
                },
            )
            .await
            .unwrap();
    }

    /// GET .../ssids/{ssidId}  →  SsidDetail
    #[tokio::test]
    async fn ssid_returns_detail() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-ssid-detail", "RT-ssid-detail")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-ssid-detail"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "ssidId": SSID_ID,
                    "name": "Corp WiFi",
                    "band": 3,
                    "security": 3,
                    "broadcast": true,
                    "vlanEnable": true,
                    "vlanId": 100,
                    "mloEnable": false,
                    "pmfMode": 2,
                    "enable11r": true,
                    "pskSetting": {
                        "versionPsk": 2,
                        "encryptionPsk": 3,
                        "gikRekeyPskEnable": false
                    }
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let detail = client.ssid(SITE_ID, WLAN_ID, SSID_ID).await.unwrap();
        assert_eq!(detail.ssid_id, SSID_ID);
        assert_eq!(detail.name, "Corp WiFi");
        assert_eq!(detail.vlan_id, Some(100));
        let psk = detail.psk_setting.as_ref().unwrap();
        assert_eq!(psk.version_psk, 2);
        assert_eq!(psk.encryption_psk, 3);
    }

    /// DELETE .../ssids/{ssidId}  →  ()
    #[tokio::test]
    async fn delete_ssid_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-del-ssid", "RT-del-ssid")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-del-ssid"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Delete SSID successfully."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client.delete_ssid(SITE_ID, WLAN_ID, SSID_ID).await.unwrap();
    }

    /// PATCH .../update-basic-config
    #[tokio::test]
    async fn update_ssid_basic_config_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-basic", "RT-basic")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-basic-config"
            )))
            .and(header("Authorization", "AccessToken=AT-basic"))
            .and(body_partial_json(serde_json::json!({ "name": "Renamed", "band": 3 })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_basic_config(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidBasicConfigRequest {
                    name: "Renamed".to_owned(),
                    band: 3,
                    guest_net_enable: false,
                    security: 0,
                    broadcast: true,
                    vlan_enable: false,
                    mlo_enable: false,
                    pmf_mode: 2,
                    enable11r: false,
                    auto_wan_access: None,
                    owe_enable: None,
                    vlan_id: None,
                    psk_setting: None,
                    ent_setting: None,
                    ppsk_setting: None,
                    hide_pwd: None,
                    gre_enable: None,
                    vlan_setting: None,
                    prohibit_wifi_share: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-wlan-schedule
    #[tokio::test]
    async fn update_ssid_wlan_schedule_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sched", "RT-sched")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-wlan-schedule"
            )))
            .and(body_partial_json(serde_json::json!({ "wlanScheduleEnable": true })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_wlan_schedule(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidWlanScheduleRequest {
                    wlan_schedule_enable: true,
                    action: Some(1),
                    schedule_id: Some("sched-001".to_owned()),
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-wifi-calling
    #[tokio::test]
    async fn update_ssid_wifi_calling_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-wcall", "RT-wcall")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-wifi-calling"
            )))
            .and(body_partial_json(serde_json::json!({ "wifiCallingEnable": false })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_wifi_calling(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidWifiCallingRequest {
                    wifi_calling_enable: false,
                    wifi_calling_id: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-rate-limit
    #[tokio::test]
    async fn update_ssid_rate_limit_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-rl", "RT-rl")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-rate-limit"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_rate_limit(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidRateLimitRequest {
                    client_rate_limit: Some(RateLimitSetting {
                        profile_id: Some("rl-profile-001".to_owned()),
                        custom_setting: None,
                    }),
                    ssid_rate_limit: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-rate-control
    #[tokio::test]
    async fn update_ssid_rate_control_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-rc", "RT-rc")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-rate-control"
            )))
            .and(body_partial_json(serde_json::json!({ "rate2gCtrlEnable": true, "rate5gCtrlEnable": false })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_rate_control(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidRateControlRequest {
                    rate2g_ctrl_enable: true,
                    rate5g_ctrl_enable: false,
                    lower_density2g: Some(6.0),
                    higher_density2g: Some(54),
                    cck_rates_disable: None,
                    client_rates_require2g: None,
                    send_beacons2g: None,
                    lower_density5g: None,
                    higher_density5g: None,
                    client_rates_require5g: None,
                    send_beacons5g: None,
                    manage_rate_control2g_enable: None,
                    manage_rate_control2g: None,
                    manage_rate_control5g_enable: None,
                    manage_rate_control5g: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-multicast-config
    #[tokio::test]
    async fn update_ssid_multicast_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-mc", "RT-mc")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-multicast-config"
            )))
            .and(body_partial_json(serde_json::json!({ "multiCastEnable": true, "channelUtil": 80 })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_multicast(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidMulticastRequest {
                    multi_cast_enable: true,
                    channel_util: 80,
                    arp_cast_enable: false,
                    ipv6_cast_enable: true,
                    filter_enable: false,
                    filter_mode: None,
                    mac_group_id: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-mac-filter
    #[tokio::test]
    async fn update_ssid_mac_filter_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-mf", "RT-mf")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-mac-filter"
            )))
            .and(body_partial_json(serde_json::json!({ "macFilterEnable": true, "policy": 1 })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_mac_filter(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidMacFilterRequest {
                    mac_filter_enable: true,
                    policy: Some(1),
                    mac_filter_id: Some("group-mac-001".to_owned()),
                    oui_profile_id_list: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-load-balance
    #[tokio::test]
    async fn update_ssid_load_balance_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-lb", "RT-lb")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-load-balance"
            )))
            .and(body_partial_json(serde_json::json!({ "enable": true })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_load_balance(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidLoadBalanceRequest { enable: true },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-hotspotv2
    #[tokio::test]
    async fn update_ssid_hotspot_v2_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-hs2", "RT-hs2")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-hotspotv2"
            )))
            .and(body_partial_json(serde_json::json!({ "hotspotV2Enable": false })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_hotspot_v2(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidHotspotV2Request {
                    hotspot_v2_enable: false,
                    network_type: None,
                    plmn_id: None,
                    roaming_consortium_oi: None,
                    operator_domain: None,
                    dgaf_disable: None,
                    he_ssid: None,
                    internet: None,
                    availability_ipv4: None,
                    availability_ipv6: None,
                    operator_friendly: None,
                    venue_info: None,
                    realm_list: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-dhcp-option
    #[tokio::test]
    async fn update_ssid_dhcp_option_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-dhcp", "RT-dhcp")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-dhcp-option"
            )))
            .and(body_partial_json(serde_json::json!({ "dhcpEnable": true })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_dhcp_option(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidDhcpOptionRequest {
                    dhcp_enable: true,
                    format: Some(0),
                    delimiter: Some(":".to_owned()),
                    circuit_id: Some(vec![3, 1, 4]),
                    remote_id: None,
                },
            )
            .await
            .unwrap();
    }

    /// PATCH .../update-band-steer
    #[tokio::test]
    async fn update_ssid_band_steer_sends_patch() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_body("AT-bs", "RT-bs")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}/update-band-steer"
            )))
            .and(body_partial_json(serde_json::json!({ "mode": 1 })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0, "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .update_ssid_band_steer(
                SITE_ID,
                WLAN_ID,
                SSID_ID,
                &UpdateSsidBandSteerRequest { mode: 1 },
            )
            .await
            .unwrap();
    }

    /// A non-zero errorCode from an SSID endpoint surfaces as Error::Api.
    #[tokio::test]
    async fn ssid_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-ssid-err", "RT-ssid-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/wireless-network/wlans/{WLAN_ID}/ssids/{SSID_ID}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33009,
                "msg": "This site template does not exist.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.ssid(SITE_ID, WLAN_ID, SSID_ID).await.unwrap_err();
        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33009),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── devices_page() / devices() ────────────────────────────────────────────

    /// GET /openapi/v1/{omadacId}/sites/{siteId}/devices
    ///   Query: page, pageSize
    ///   Returns: Page<DeviceInfo>
    #[tokio::test]
    async fn devices_page_returns_page() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-devices-page", "RT-devices-page")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/devices"
            )))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "10"))
            .and(header("Authorization", "AccessToken=AT-devices-page"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 2,
                    "currentPage": 1,
                    "currentSize": 10,
                    "data": [
                        {
                            "mac": "AA-BB-CC-DD-EE-01",
                            "name": "AP-Lobby",
                            "type": "ap",
                            "model": "EAP670",
                            "ip": "192.168.1.101",
                            "status": 1
                        },
                        {
                            "mac": "AA-BB-CC-DD-EE-02",
                            "name": "SW-Core",
                            "type": "switch",
                            "model": "TL-SG3428X",
                            "ip": "192.168.1.1",
                            "status": 1
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client.devices_page(SITE_ID, 1, 10, None).await.unwrap();

        assert_eq!(page.total_rows, 2);
        assert_eq!(page.data.len(), 2);
        assert_eq!(page.data[0].mac.as_deref(), Some("AA-BB-CC-DD-EE-01"));
        assert_eq!(page.data[0].name.as_deref(), Some("AP-Lobby"));
        assert_eq!(page.data[1].mac.as_deref(), Some("AA-BB-CC-DD-EE-02"));
    }

    /// devices_page forwards optional search/sort/filter query params.
    #[tokio::test]
    async fn devices_page_forwards_optional_params() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-devices-opt", "RT-devices-opt")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/devices"
            )))
            .and(query_param("searchKey", "lobby"))
            .and(query_param("sorts.name", "asc"))
            .and(query_param("filters.tag", "wifi"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 0,
                    "currentPage": 1,
                    "currentSize": 10,
                    "data": []
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let params = DeviceListParams {
            search_key: Some("lobby".to_owned()),
            sort_name: Some("asc".to_owned()),
            filter_tag: Some("wifi".to_owned()),
            ..Default::default()
        };

        let page = client
            .devices_page(SITE_ID, 1, 10, Some(&params))
            .await
            .unwrap();

        assert_eq!(page.total_rows, 0);
    }

    /// A non-zero errorCode from the devices endpoint surfaces as Error::Api.
    #[tokio::test]
    async fn devices_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-devices-err", "RT-devices-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/devices"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -1001,
                "msg": "Invalid request parameters.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.devices_page(SITE_ID, 1, 10, None).await.unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -1001),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── lan_networks_page() / lan_networks() ──────────────────────────────────

    /// GET /openapi/v3/{omadacId}/sites/{siteId}/lan-networks
    ///   Query: page, pageSize
    ///   Returns: LanNetworkPage (data + capability metadata)
    #[tokio::test]
    async fn lan_networks_page_returns_page_with_metadata() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-lan-page", "RT-lan-page")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v3/{OMADAC_ID}/sites/{SITE_ID}/lan-networks"
            )))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "10"))
            .and(header("Authorization", "AccessToken=AT-lan-page"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 2,
                    "currentPage": 1,
                    "currentSize": 10,
                    "data": [
                        {
                            "id": "net-001",
                            "name": "Default",
                            "purpose": 1,
                            "vlan": 1,
                            "igmpSnoopEnable": false,
                            "deviceType": 1,
                            "gatewaySubnet": "192.168.1.1/24"
                        },
                        {
                            "id": "net-002",
                            "name": "IoT",
                            "purpose": 0,
                            "vlan": 20,
                            "igmpSnoopEnable": false,
                            "deviceType": 1,
                            "gatewaySubnet": "10.0.20.1/24"
                        }
                    ],
                    "supportMultiVlan": true,
                    "supportRA": false,
                    "supportCustomDhcpOption": true,
                    "dhcpRangePoolSize": 8,
                    "vlanNums": 2
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client.lan_networks_page(SITE_ID, 1, 10).await.unwrap();

        assert_eq!(page.total_rows, 2);
        assert_eq!(page.data.len(), 2);
        assert_eq!(page.data[0].id.as_deref(), Some("net-001"));
        assert_eq!(page.data[0].name, "Default");
        assert_eq!(page.data[1].vlan, Some(20));
        assert_eq!(page.support_multi_vlan, Some(true));
        assert_eq!(page.support_ra, Some(false));
        assert_eq!(page.dhcp_range_pool_size, Some(8));
        assert_eq!(page.vlan_nums, Some(2));
    }

    /// A non-zero errorCode from the lan-networks endpoint surfaces as Error::Api.
    #[tokio::test]
    async fn lan_networks_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-lan-err", "RT-lan-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v3/{OMADAC_ID}/sites/{SITE_ID}/lan-networks"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33000,
                "msg": "This site does not exist.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.lan_networks_page(SITE_ID, 1, 10).await.unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33000),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    // ── lan_profiles_page() / create_lan_profile() / delete_lan_profile() / update_lan_profile() ──

    fn lan_profile_body() -> serde_json::Value {
        serde_json::json!({
            "id": "profile-001",
            "flag": 2,
            "name": "Office Ports",
            "poe": 0,
            "nativeNetworkId": "net-001",
            "tagNetworkIds": [],
            "untagNetworkIds": [],
            "dot1x": 1,
            "portIsolationEnable": false,
            "lldpMedEnable": true,
            "bandWidthCtrlType": 0,
            "spanningTreeEnable": true,
            "loopbackDetectEnable": false
        })
    }

    fn lan_profile_request_body() -> serde_json::Value {
        serde_json::json!({
            "name": "Office Ports",
            "poe": 0,
            "nativeNetworkId": "net-001",
            "dot1x": 1,
            "portIsolationEnable": false,
            "lldpMedEnable": true,
            "bandWidthCtrlType": 0,
            "spanningTreeEnable": true,
            "loopbackDetectEnable": false
        })
    }

    /// GET /openapi/v1/{omadacId}/sites/{siteId}/lan-profiles
    ///   Query: page, pageSize
    ///   Returns: Page<LanProfile>
    #[tokio::test]
    async fn lan_profiles_page_returns_page() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-lanp-page", "RT-lanp-page")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/lan-profiles"
            )))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "10"))
            .and(header("Authorization", "AccessToken=AT-lanp-page"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 1,
                    "currentPage": 1,
                    "currentSize": 10,
                    "data": [lan_profile_body()]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client.lan_profiles_page(SITE_ID, 1, 10).await.unwrap();

        assert_eq!(page.total_rows, 1);
        assert_eq!(page.data.len(), 1);
        assert_eq!(page.data[0].id.as_deref(), Some("profile-001"));
        assert_eq!(page.data[0].name, "Office Ports");
        assert_eq!(page.data[0].poe, 0);
    }

    /// POST /openapi/v1/{omadacId}/sites/{siteId}/lan-profiles
    ///   Body: LanProfileConfig
    ///   Returns: id of new profile
    #[tokio::test]
    async fn create_lan_profile_posts_body_and_returns_id() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-lanp-create", "RT-lanp-create")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/lan-profiles"
            )))
            .and(header("Authorization", "AccessToken=AT-lanp-create"))
            .and(body_partial_json(lan_profile_request_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": { "id": "profile-new" }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = LanProfileConfig {
            name: "Office Ports".to_owned(),
            poe: 0,
            native_network_id: "net-001".to_owned(),
            tag_network_ids: None,
            untag_network_ids: None,
            voice_network_id: None,
            dot1x: 1,
            port_isolation_enable: false,
            lldp_med_enable: true,
            band_width_ctrl_type: 0,
            storm_ctrl: None,
            band_ctrl: None,
            spanning_tree_enable: true,
            spanning_tree_setting: None,
            loopback_detect_enable: false,
            eee_enable: None,
            flow_control_enable: None,
            loopback_detect_vlan_based_enable: None,
            igmp_fast_leave_enable: None,
            mld_fast_leave_enable: None,
            dhcp_l2_relay_settings: None,
            fast_leave_enable: None,
            dot1p_priority: None,
            trust_mode: None,
        };

        let id = client.create_lan_profile(SITE_ID, &body).await.unwrap();
        assert_eq!(id, "profile-new");
    }

    /// DELETE /openapi/v1/{omadacId}/sites/{siteId}/lan-profiles/{profileId}
    #[tokio::test]
    async fn delete_lan_profile_sends_delete_and_succeeds() {
        let server = MockServer::start().await;
        const PROFILE_ID: &str = "profile-001";

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-lanp-del", "RT-lanp-del")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/lan-profiles/{PROFILE_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-lanp-del"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .delete_lan_profile(SITE_ID, PROFILE_ID)
            .await
            .unwrap();
    }

    /// PATCH /openapi/v1/{omadacId}/sites/{siteId}/lan-profiles/{profileId}
    ///   Body: LanProfileConfig
    #[tokio::test]
    async fn update_lan_profile_sends_patch_and_succeeds() {
        let server = MockServer::start().await;
        const PROFILE_ID: &str = "profile-001";

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-lanp-patch", "RT-lanp-patch")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/lan-profiles/{PROFILE_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-lanp-patch"))
            .and(body_partial_json(lan_profile_request_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = LanProfileConfig {
            name: "Office Ports".to_owned(),
            poe: 0,
            native_network_id: "net-001".to_owned(),
            tag_network_ids: None,
            untag_network_ids: None,
            voice_network_id: None,
            dot1x: 1,
            port_isolation_enable: false,
            lldp_med_enable: true,
            band_width_ctrl_type: 0,
            storm_ctrl: None,
            band_ctrl: None,
            spanning_tree_enable: true,
            spanning_tree_setting: None,
            loopback_detect_enable: false,
            eee_enable: None,
            flow_control_enable: None,
            loopback_detect_vlan_based_enable: None,
            igmp_fast_leave_enable: None,
            mld_fast_leave_enable: None,
            dhcp_l2_relay_settings: None,
            fast_leave_enable: None,
            dot1p_priority: None,
            trust_mode: None,
        };

        client
            .update_lan_profile(SITE_ID, PROFILE_ID, &body)
            .await
            .unwrap();
    }

    /// A non-zero errorCode from the lan-profiles endpoint surfaces as Error::Api.
    #[tokio::test]
    async fn lan_profiles_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-lanp-err", "RT-lanp-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/lan-profiles"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33507,
                "msg": "This profile does not exist.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.lan_profiles_page(SITE_ID, 1, 10).await.unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33507),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    const SWITCH_MAC: &str = "AA-BB-CC-DD-EE-FF";

    /// GET switch info returns a populated SwitchInfo.
    #[tokio::test]
    async fn switch_info_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-info", "RT-sw-info")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "mac": "AA-BB-CC-DD-EE-FF",
                    "ip": "192.168.1.2",
                    "ipv6List": [],
                    "model": "TL-SG3428X",
                    "firmwareVersion": "3.0.0 Build 20230101 Rel. 12345",
                    "version": "3.0.0",
                    "hwVersion": "1.0",
                    "cpuUtil": 5,
                    "memUtil": 42,
                    "uptime": "10 days 3 hours",
                    "portList": [
                        {
                            "port": 1,
                            "name": "Port1",
                            "profileId": "prof-001",
                            "profileName": "Default",
                            "profileOverrideEnable": false,
                            "poeMode": 1,
                            "lagPort": false,
                            "status": 1
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let info = client.switch(SITE_ID, SWITCH_MAC).await.unwrap();

        assert_eq!(info.mac.as_deref(), Some("AA-BB-CC-DD-EE-FF"));
        assert_eq!(info.ip.as_deref(), Some("192.168.1.2"));
        assert_eq!(info.model.as_deref(), Some("TL-SG3428X"));
        assert_eq!(info.cpu_util, Some(5));
        assert_eq!(info.mem_util, Some(42));
        let ports = info.port_list.unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 1);
        assert_eq!(ports[0].profile_id, "prof-001");
        assert!(!ports[0].lag_port);
        assert_eq!(ports[0].status, 1);
    }

    /// A non-zero errorCode from switch info surfaces as Error::Api.
    #[tokio::test]
    async fn switch_info_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sw-info-err", "RT-sw-info-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -39050,
                "msg": "This device does not exist.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client.switch(SITE_ID, SWITCH_MAC).await.unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -39050),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    const PORT: &str = "1";
    const PROFILE_ID: &str = "prof-abc";

    /// PATCH switch port sends body and succeeds.
    #[tokio::test]
    async fn update_switch_port_sends_body() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-port", "RT-sw-port")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/ports/{PORT}"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-port"))
            .and(body_partial_json(serde_json::json!({"name": "Uplink"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = SwitchPortConfig {
            name: Some("Uplink".to_owned()),
            disable: Some(false),
            profile_id: None,
            profile_override_enable: None,
            profile_vlan_override_enable: None,
            tag_ids: None,
            native_network_id: None,
            native_bridge_vlan: None,
            network_tags_setting: None,
            tag_network_ids: None,
            untag_network_ids: None,
            voice_network_enable: None,
            voice_network_id: None,
            voice_bridge_vlan: None,
            voice_dscp_enable: None,
            voice_dscp: None,
            port_alert_enable: None,
            fec_mode: None,
            fec_link_peer_apply_enable: None,
            link_speed: None,
            duplex: None,
            igmp_snooping_enable: None,
            band_width_ctrl_type: None,
            band_ctrl: None,
            storm_ctrl: None,
            spanning_tree_enable: None,
            spanning_tree_setting: None,
            loopback_detect_enable: None,
            loopback_detect_vlan_based_enable: None,
            igmp_fast_leave_enable: None,
            mld_fast_leave_enable: None,
            dhcp_snoop_enable: None,
            arp_detect_enable: None,
            impbs: None,
            port_isolation_enable: None,
            eee_enable: None,
            flow_control_enable: None,
            fast_leave_enable: None,
            dhcp_l2_relay_settings: None,
            dot1p_priority: None,
            trust_mode: None,
            qos_queue_enable: None,
            queue_id: None,
            operation: None,
            mirrored_ports: None,
            mirrored_lags: None,
            lag_setting: None,
            dot1x: None,
            poe: None,
            lldp_med_enable: None,
            topo_notify_enable: None,
        };

        client
            .update_switch_port(SITE_ID, SWITCH_MAC, PORT, &body)
            .await
            .unwrap();
    }

    /// A non-zero errorCode from update_switch_port surfaces as Error::Api.
    #[tokio::test]
    async fn update_switch_port_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sw-port-err", "RT-sw-port-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/ports/{PORT}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -39701,
                "msg": "This port does not exist",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = SwitchPortConfig {
            name: None,
            disable: Some(true),
            profile_id: None,
            profile_override_enable: None,
            profile_vlan_override_enable: None,
            tag_ids: None,
            native_network_id: None,
            native_bridge_vlan: None,
            network_tags_setting: None,
            tag_network_ids: None,
            untag_network_ids: None,
            voice_network_enable: None,
            voice_network_id: None,
            voice_bridge_vlan: None,
            voice_dscp_enable: None,
            voice_dscp: None,
            port_alert_enable: None,
            fec_mode: None,
            fec_link_peer_apply_enable: None,
            link_speed: None,
            duplex: None,
            igmp_snooping_enable: None,
            band_width_ctrl_type: None,
            band_ctrl: None,
            storm_ctrl: None,
            spanning_tree_enable: None,
            spanning_tree_setting: None,
            loopback_detect_enable: None,
            loopback_detect_vlan_based_enable: None,
            igmp_fast_leave_enable: None,
            mld_fast_leave_enable: None,
            dhcp_snoop_enable: None,
            arp_detect_enable: None,
            impbs: None,
            port_isolation_enable: None,
            eee_enable: None,
            flow_control_enable: None,
            fast_leave_enable: None,
            dhcp_l2_relay_settings: None,
            dot1p_priority: None,
            trust_mode: None,
            qos_queue_enable: None,
            queue_id: None,
            operation: None,
            mirrored_ports: None,
            mirrored_lags: None,
            lag_setting: None,
            dot1x: None,
            poe: None,
            lldp_med_enable: None,
            topo_notify_enable: None,
        };

        let err = client
            .update_switch_port(SITE_ID, SWITCH_MAC, PORT, &body)
            .await
            .unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -39701),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    /// PUT switch port profile sends correct profileId.
    #[tokio::test]
    async fn set_switch_port_profile_sends_profile_id() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-prof", "RT-sw-prof")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/ports/{PORT}/profile"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-prof"))
            .and(body_partial_json(
                serde_json::json!({"profileId": PROFILE_ID}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        client
            .set_switch_port_profile(SITE_ID, SWITCH_MAC, PORT, PROFILE_ID)
            .await
            .unwrap();
    }

    /// A non-zero errorCode from set_switch_port_profile surfaces as Error::Api.
    #[tokio::test]
    async fn set_switch_port_profile_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sw-prof-err", "RT-sw-prof-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/ports/{PORT}/profile"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33507,
                "msg": "This profile does not exist.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client
            .set_switch_port_profile(SITE_ID, SWITCH_MAC, PORT, PROFILE_ID)
            .await
            .unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33507),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    const NETWORK_ID: &str = "net-vlan10";

    /// GET switch networks page returns a populated SwitchNetworksPage.
    #[tokio::test]
    async fn switch_networks_page_returns_page() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-nets", "RT-sw-nets")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/networks"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-nets"))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "totalRows": 1,
                    "currentPage": 1,
                    "currentSize": 10,
                    "supportIpv6": true,
                    "data": [
                        {
                            "id": NETWORK_ID,
                            "vlan": 10,
                            "status": true,
                            "mvlan": true,
                            "name": "Management",
                            "mode": 1,
                            "dhcpServer": {
                                "ip": "192.168.10.1",
                                "netmask": "255.255.255.0",
                                "leasetime": 120
                            }
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let page = client
            .switch_networks_page(SITE_ID, SWITCH_MAC, 1, 10)
            .await
            .unwrap();

        assert_eq!(page.total_rows, 1);
        assert_eq!(page.support_ipv6, Some(true));
        assert_eq!(page.data.len(), 1);
        let net = &page.data[0];
        assert_eq!(net.id, NETWORK_ID);
        assert_eq!(net.vlan, 10);
        assert!(net.mvlan);
        assert_eq!(net.mode, 1);
        let srv = net.dhcp_server.as_ref().unwrap();
        assert_eq!(srv.ip, "192.168.10.1");
        assert_eq!(srv.leasetime, 120);
    }

    /// A non-zero errorCode from switch networks surfaces as Error::Api.
    #[tokio::test]
    async fn switch_networks_page_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sw-nets-err", "RT-sw-nets-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/networks"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -39700,
                "msg": "Switch does not exist",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client
            .switch_networks_page(SITE_ID, SWITCH_MAC, 1, 10)
            .await
            .unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -39700),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    /// POST switch network modifies the VLAN interface config.
    #[tokio::test]
    async fn update_switch_network_sends_body() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sw-net-put", "RT-sw-net-put")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/networks/{NETWORK_ID}"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-net-put"))
            .and(body_partial_json(serde_json::json!({"id": NETWORK_ID, "vlan": 10})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = SwitchNetwork {
            id: NETWORK_ID.to_owned(),
            vlan: 10,
            status: Some(true),
            mvlan: true,
            name: None,
            ip: None,
            ipv6_enable: None,
            ipv6: None,
            mode: 0,
            dhcp_server: None,
            dhcp_relay: None,
            vrf_id: None,
        };

        client
            .update_switch_network(SITE_ID, SWITCH_MAC, NETWORK_ID, &body)
            .await
            .unwrap();
    }

    /// A non-zero errorCode from update_switch_network surfaces as Error::Api.
    #[tokio::test]
    async fn update_switch_network_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body("AT-sw-net-err", "RT-sw-net-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/networks/{NETWORK_ID}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -33529,
                "msg": "The network cannot be disabled because it has been configured as the Management VLAN.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = SwitchNetwork {
            id: NETWORK_ID.to_owned(),
            vlan: 10,
            status: Some(false),
            mvlan: true,
            name: None,
            ip: None,
            ipv6_enable: None,
            ipv6: None,
            mode: 0,
            dhcp_server: None,
            dhcp_relay: None,
            vrf_id: None,
        };

        let err = client
            .update_switch_network(SITE_ID, SWITCH_MAC, NETWORK_ID, &body)
            .await
            .unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -33529),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    /// GET general-config returns a populated SwitchGeneralConfig.
    #[tokio::test]
    async fn switch_general_config_returns_config() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-get", "RT-sw-get")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/general-config"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-get"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success.",
                "result": {
                    "name": "Core Switch",
                    "ledSetting": 1,
                    "tagIds": ["tag-001"],
                    "jumbo": 9216,
                    "lagHashAlg": 2
                }
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let cfg = client
            .switch_general_config(SITE_ID, SWITCH_MAC)
            .await
            .unwrap();

        assert_eq!(cfg.name.as_deref(), Some("Core Switch"));
        assert_eq!(cfg.led_setting, Some(1));
        assert_eq!(cfg.jumbo, Some(9216));
        assert_eq!(cfg.lag_hash_alg, Some(2));
    }

    /// PATCH general-config succeeds.
    #[tokio::test]
    async fn update_switch_general_config_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-patch", "RT-sw-patch")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/general-config"
            )))
            .and(header("Authorization", "AccessToken=AT-sw-patch"))
            .and(body_partial_json(
                serde_json::json!({"name": "Core Switch"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": 0,
                "msg": "Success."
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let body = SwitchGeneralConfig {
            name: Some("Core Switch".to_owned()),
            led_setting: None,
            tag_ids: None,
            location: None,
            jumbo: None,
            lag_hash_alg: None,
            sdm: None,
        };

        client
            .update_switch_general_config(SITE_ID, SWITCH_MAC, &body)
            .await
            .unwrap();
    }

    /// A non-zero errorCode from switch general-config surfaces as Error::Api.
    #[tokio::test]
    async fn switch_general_config_api_error_returns_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openapi/authorize/token"))
            .and(query_param("grant_type", "client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_body("AT-sw-err", "RT-sw-err")),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!(
                "/openapi/v1/{OMADAC_ID}/sites/{SITE_ID}/switches/{SWITCH_MAC}/general-config"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errorCode": -39700,
                "msg": "The switch does not exist.",
                "result": null
            })))
            .mount(&server)
            .await;

        let client =
            OmadaClient::with_client_credentials(server.uri(), OMADAC_ID, CLIENT_ID, CLIENT_SECRET)
                .await
                .unwrap();

        let err = client
            .switch_general_config(SITE_ID, SWITCH_MAC)
            .await
            .unwrap_err();

        match err {
            Error::Api { error_code, .. } => assert_eq!(error_code, -39700),
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }
}

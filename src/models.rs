// Public types returned to consumers of this crate.
// Re-exported at the crate root via `pub use models::*`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A paginated result set returned by list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    /// Total rows of all items.
    pub total_rows: i64,
    /// Current page number.
    pub current_page: i32,
    /// Number of entries per page.
    pub current_size: i32,
    /// Items on this page.
    pub data: Vec<T>,
}

/// Summary information for a site.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    /// Site ID.
    pub site_id: String,
    /// Name of the site should contain 1 to 64 characters.
    pub name: String,
    /// Site tag IDs.
    pub tag_ids: Option<Vec<String>>,
    /// Country/Region of the site. Refer to the abbreviation of the ISO country
    /// code (e.g. `"United States"` for the United States of America).
    pub region: Option<String>,
    /// Timezone of the site. For valid values refer to section 5.1 of the Open
    /// API Access Guide.
    pub time_zone: Option<String>,
    /// Scenario of the site. For valid values refer to the Get scenario list
    /// endpoint.
    pub scenario: Option<String>,
    /// Longitude of the site. Must be within the range of -180 to 180.
    pub longitude: Option<f64>,
    /// Latitude of the site. Must be within the range of -90 to 90.
    pub latitude: Option<f64>,
    /// Address of the site.
    pub address: Option<String>,
    /// Site type (Pro controller only). `0`: Basic Site; `1`: Pro Site.
    #[serde(rename = "type")]
    pub site_type: Option<i32>,
    /// Whether the site supports adopting Agile Series Switches.
    pub support_es: Option<bool>,
    /// Whether the site supports adopting Non-Agile Series Switches.
    pub support_l2: Option<bool>,
    /// Adopted gateway public IP of the site. Only applicable for cloud-based
    /// controllers and remote-management local controllers.
    pub site_public_ip: Option<String>,
    /// Whether this is the default site.
    pub primary: Option<bool>,
}

/// A WLAN group belonging to a site.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WlanGroup {
    /// WLAN group ID.
    pub wlan_id: String,
    /// WLAN group name (1 to 128 characters).
    pub name: String,
    /// Whether this is the default WLAN group.
    pub primary: Option<bool>,
    /// Whether this group was cloned from another.
    pub clone: Option<bool>,
    /// ID of the WLAN group this was cloned from.
    pub clone_wlan_id: Option<String>,
    /// Site ID this group belongs to.
    pub site: Option<String>,
    /// Resource origin: `0` = newly created, `1` = from template, `2` = override template.
    pub resource: Option<i32>,
}

/// Request body for [`OmadaClient::create_wlan_group`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWlanGroupRequest {
    /// WLAN group name (1 to 128 characters).
    pub name: String,
    /// Whether to clone the SSID list from another WLAN group.
    pub clone: bool,
    /// ID of the WLAN group to clone from. Required when `clone` is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloned_wlan_id: Option<String>,
}

// ── Group profiles ────────────────────────────────────────────────────────────

/// Type of a group profile.
///
/// `0`: IP Group; `1`: IP Port Group; `2`: MAC Group; `3`: IPv6 Group;
/// `4`: IPv6 Port Group; `5`: Country Group; `7`: Domain Group.
pub type GroupProfileType = i32;

/// An IP subnet entry used in IP Group and IP Port Group profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpSubnet {
    /// IP address, should be a valid IP format.
    pub ip: String,
    /// IP mask, must be within the range of 1–32.
    pub mask: i32,
    /// Optional description (1 to 512 characters).
    pub description: Option<String>,
}

/// An IPv6 subnet entry used in IPv6 Group and IPv6 Port Group profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6Subnet {
    /// IPv6 address, should be a valid IPv6 format.
    pub ip: String,
    /// IPv6 prefix, must be within the range of 1–128.
    pub prefix: i32,
    /// Optional description (1 to 512 characters).
    pub description: Option<String>,
}

/// A port mask entry. Valid when `port_type` is `1`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortMask {
    /// Port, must be within the range of 0–65535.
    pub port: i32,
    /// Port mask: four hex digits (`0`–`9`, `A`–`F`), e.g. `"0000"` or `"FFFF"`.
    pub mask: String,
}

/// A MAC address entry used in MAC Group profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacAddress {
    /// ID of the MAC address entry.
    pub rule_id: Option<i32>,
    /// MAC address name.
    pub name: Option<String>,
    /// MAC address value.
    pub mac_address: String,
}

/// A domain entry used in Domain Group profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    /// Domain address, should be a valid domain address.
    pub address: String,
    /// Domain port, within the range of 0–65535 or empty, e.g. `"80"` or `"80-100"`.
    pub port: Option<String>,
    /// Optional description (1 to 512 characters).
    pub description: Option<String>,
}

/// A group profile returned by the [`OmadaClient::group_profiles`] endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupProfile {
    /// Group profile ID.
    pub group_id: String,
    /// Group profile name.
    pub name: String,
    /// Type of group profile. `0`: IP Group; `1`: IP Port Group; `2`: MAC Group;
    /// `3`: IPv6 Group; `4`: IPv6 Port Group; `5`: Country Group; `7`: Domain Group.
    #[serde(rename = "type")]
    pub group_type: GroupProfileType,
    /// Count of list entries.
    pub count: Option<i32>,
    /// Whether this is a built-in profile.
    pub build_in: Option<bool>,
    /// IP subnet info list. Required when `group_type` is `0` or `1`.
    pub ip_list: Option<Vec<IpSubnet>>,
    /// IPv6 subnet info list. Required when `group_type` is `3` or `4`.
    pub ipv6_list: Option<Vec<Ipv6Subnet>>,
    /// Port type. `0`: port range; `1`: port mask. Valid when `group_type` is `1` or `4`.
    pub port_type: Option<i32>,
    /// Port list. Valid when `port_type` is `0`.
    pub port_list: Option<Vec<String>>,
    /// Port mask list. Valid when `port_type` is `1`.
    pub port_mask_list: Option<Vec<PortMask>>,
    /// MAC address list. Valid when `group_type` is `2`.
    pub mac_address_list: Option<Vec<MacAddress>>,
    /// Country list. Valid when `group_type` is `5`.
    pub country_list: Option<Vec<String>>,
    /// Description. Valid when `group_type` is `5`.
    pub description: Option<String>,
    /// Domain name list (deprecated). Valid when `group_type` is `7`.
    #[deprecated]
    pub domain_name: Option<Vec<String>>,
    /// Domain info including optional ports. Valid when `group_type` is `7`.
    pub domain_name_port: Option<Vec<Domain>>,
}

// ── SSID sub-types ────────────────────────────────────────────────────────────

/// WPA-Personal security settings for an SSID. Required when `security` is `3`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidPskSetting {
    /// WPA-Personal password (8–63 printable ASCII or hex digits). `None` when
    /// the password is hidden (`hide_pwd` is `true`).
    pub security_key: Option<String>,
    /// WPA-Personal version. `1`: WPA-PSK; `2`: WPA2-PSK; `3`: WPA/WPA2-PSK;
    /// `4`: WPA3-SAE.
    pub version_psk: i32,
    /// WPA-Personal encryption. `1`: Auto; `3`: AES.
    pub encryption_psk: i32,
    /// Whether to enable the group key update period.
    pub gik_rekey_psk_enable: bool,
    /// Group key update interval value.
    pub rekey_psk_interval: Option<i32>,
    /// Group key update interval unit. `0`: Seconds; `1`: Minutes; `2`: Hours.
    pub interval_psk_type: Option<i32>,
}

/// WPA-Enterprise security settings for an SSID. Required when `security` is `2`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidEnterpriseSetting {
    /// RADIUS Profile ID.
    pub radius_profile_id: String,
    /// WPA-Enterprise version. `1`: WPA-Enterprise; `2`: WPA2-Enterprise;
    /// `3`: WPA/WPA2-Enterprise; `4`: WPA3-Enterprise.
    pub version_ent: i32,
    /// WPA-Enterprise encryption. `1`: Auto; `3`: AES; `4`: AES-GCM 256;
    /// `5`: AES-CNSA; `6`: `CCMP_128`.
    pub encryption_ent: i32,
    /// Whether to enable the group key update period.
    pub gik_rekey_ent_enable: bool,
    /// Group key update interval value.
    pub rekey_ent_interval: Option<i32>,
    /// Group key update interval unit. `0`: Seconds; `1`: Minutes; `2`: Hours.
    pub interval_ent_type: Option<i32>,
    /// NAS ID mode. `0`: default; `1`: follow device name; `2`: custom.
    pub nas_id_mode: Option<i32>,
    /// Custom NAS ID. Required when `nas_id_mode` is `2`.
    pub nas_id: Option<String>,
}

/// PPSK security settings for an SSID. Required when `security` is `4` or `5`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidPpskSetting {
    /// PPSK Profile ID. Required when `security` is `4` (PPSK without RADIUS).
    pub ppsk_profile_id: Option<String>,
    /// RADIUS Profile ID. Required when `security` is `5` (PPSK with RADIUS).
    pub radius_profile_id: Option<String>,
    /// MAC address format. `0`–`5` (various formatting styles). Required when
    /// `security` is `5`.
    pub mac_format: Option<i32>,
    /// NAS ID (1–64 characters). Required when `security` is `5`.
    pub nas_id: Option<String>,
    /// Authentication type. `0`: MAC Auth; `1`: EKMS; `2`: Generic Radius with
    /// unbound MAC. Required when `security` is `5`.
    #[serde(rename = "type")]
    pub ppsk_type: Option<i32>,
}

/// Custom VLAN assignment config within an SSID VLAN setting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidVlanCustomSetting {
    /// Custom mode. `0`: by Network; `1`: by VLAN.
    pub custom_mode: i32,
    /// LAN network ID. Required when `custom_mode` is `0`. Mutually exclusive
    /// with `lan_network_vlan_ids`.
    pub lan_network_id: Option<String>,
    /// Bridge VLAN. Used when `custom_mode` is `0` and VLAN pool is not in use.
    pub bridge_vlan: Option<i32>,
    /// VLAN ID. Required when `custom_mode` is `1`. Mutually exclusive with
    /// `vlan_pool_ids`.
    pub vlan_id: Option<i32>,
    /// Mapping of LAN network ID to VLAN IDs. Used when VLAN pool is enabled
    /// and `custom_mode` is `0`.
    pub lan_network_vlan_ids: Option<HashMap<String, Vec<i32>>>,
    /// VLAN pool IDs string. Used when VLAN pool is enabled and `custom_mode`
    /// is `1`.
    pub vlan_pool_ids: Option<String>,
}

/// VLAN configuration attached to an SSID. Required when `vlan_enable` is
/// `true` (alternative to the plain `vlan_id` field).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidVlanSetting {
    /// VLAN mode. `0`: Default; `1`: Custom.
    pub mode: i32,
    /// Custom VLAN config. Required when `mode` is `1`.
    pub custom_config: Option<SsidVlanCustomSetting>,
}

/// Per-client or per-SSID rate limit configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitCustomSetting {
    /// Whether to limit downlink speed.
    pub down_limit_enable: bool,
    /// Downlink speed limit value (Kbps or Mbps depending on `down_limit_type`).
    pub down_limit: Option<i32>,
    /// Downlink speed limit unit. `0`: Kbps; `1`: Mbps.
    pub down_limit_type: Option<i32>,
    /// Whether to limit uplink speed.
    pub up_limit_enable: bool,
    /// Uplink speed limit value (Kbps or Mbps depending on `up_limit_type`).
    pub up_limit: Option<i32>,
    /// Uplink speed limit unit. `0`: Kbps; `1`: Mbps.
    pub up_limit_type: Option<i32>,
}

/// Rate limit configuration referencing a profile or custom settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSetting {
    /// Rate limit profile ID. Takes precedence over `custom_setting`.
    pub profile_id: Option<String>,
    /// Custom rate limit values used when no profile is referenced.
    pub custom_setting: Option<RateLimitCustomSetting>,
}

/// SSID WLAN schedule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidWlanSchedule {
    /// Whether the WLAN schedule is enabled.
    pub wlan_schedule_enable: bool,
    /// Schedule action. `0`: radio off during selected period; `1`: radio on
    /// during selected period.
    pub action: Option<i32>,
    /// Time Range Profile ID.
    pub schedule_id: Option<String>,
}

/// SSID 802.11 data rate control configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidRateControl {
    /// Whether 2.4 GHz data rate control is enabled.
    pub rate2g_ctrl_enable: Option<bool>,
    /// 2.4 GHz lower density threshold in Mbps (e.g. `1.0`, `5.5`, `11.0`).
    pub lower_density2g: Option<f64>,
    /// 2.4 GHz higher density threshold in Mbps (fixed at `54`).
    pub higher_density2g: Option<i32>,
    /// Whether to disable 2G CCK rates.
    pub cck_rates_disable: Option<bool>,
    /// Whether to require clients to use rates at or above the 2.4 GHz threshold.
    pub client_rates_require2g: Option<bool>,
    /// Whether to send beacons at 1 Mbps on 2.4 GHz.
    pub send_beacons2g: Option<bool>,
    /// Whether 5 GHz data rate control is enabled.
    pub rate5g_ctrl_enable: Option<bool>,
    /// 5 GHz lower density threshold in Mbps.
    pub lower_density5g: Option<i32>,
    /// 5 GHz higher density threshold in Mbps (fixed at `54`).
    pub higher_density5g: Option<i32>,
    /// Whether to require clients to use rates at or above the 5 GHz threshold.
    pub client_rates_require5g: Option<bool>,
    /// Whether to send beacons at 6 Mbps on 5 GHz.
    pub send_beacons5g: Option<bool>,
    /// Whether 6 GHz data rate control is enabled.
    pub rate6g_ctrl_enable: Option<bool>,
    /// 6 GHz lower density threshold in Mbps.
    pub lower_density6g: Option<i32>,
    /// 6 GHz higher density threshold in Mbps (fixed at `54`).
    pub higher_density6g: Option<i32>,
    /// Whether to require clients to use rates at or above the 6 GHz threshold.
    pub client_rates_require6g: Option<bool>,
    /// Whether to send beacons at 6 Mbps on 6 GHz.
    pub send_beacons6g: Option<bool>,
    /// Whether 2.4 GHz manage rate control is enabled.
    pub manage_rate_control2g_enable: Option<bool>,
    /// 2.4 GHz manage rate control lower density value in Mbps.
    pub manage_rate_control2g: Option<f64>,
    /// Whether 5 GHz manage rate control is enabled.
    pub manage_rate_control5g_enable: Option<bool>,
    /// 5 GHz manage rate control lower density value in Mbps.
    pub manage_rate_control5g: Option<i32>,
}

/// SSID MAC filter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidMacFilter {
    /// Whether MAC filtering is enabled.
    pub mac_filter_enable: Option<bool>,
    /// Filter policy. `0`: Deny List; `1`: Allow List.
    pub policy: Option<i32>,
    /// MAC Group Profile ID.
    pub mac_filter_id: Option<String>,
    /// OUI Profile ID list.
    pub oui_profile_id_list: Option<Vec<String>>,
}

/// SSID multicast/broadcast management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidMulticast {
    /// Whether multicast-to-unicast conversion is enabled.
    pub multi_cast_enable: Option<bool>,
    /// Channel utilisation threshold (0–100) above which multicast is not
    /// converted to unicast.
    pub channel_util: Option<i32>,
    /// Whether ARP-to-unicast conversion is enabled.
    pub arp_cast_enable: Option<bool>,
    /// Whether IPv6 multicast-to-unicast conversion is enabled.
    pub ipv6_cast_enable: Option<bool>,
    /// Whether the multicast filter is enabled.
    pub filter_enable: Option<bool>,
    /// Bitmask of enabled filter protocols. Bit 0: IGMP; bit 1: mDNS; bit 2:
    /// Others.
    pub filter_mode: Option<i32>,
    /// MAC Group Profile ID used as the multicast filter.
    pub mac_group_id: Option<String>,
}

/// SSID DHCP Option 82 configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidDhcpOption {
    /// Whether DHCP Option 82 is enabled.
    pub dhcp_enable: Option<bool>,
    /// Option 82 encoding format. `0`: ASCII; `1`: Binary.
    pub format: Option<i32>,
    /// Option 82 delimiter (a single printable ASCII character).
    pub delimiter: Option<String>,
    /// Circuit-ID element order. Each integer corresponds to: `1`: VLAN-ID;
    /// `2`: AP Radio MAC; `3`: SSID-Type; `4`: SSID-Name; `5`: AP Ethernet
    /// MAC; `6`: Site-Name.
    pub circuit_id: Option<Vec<i32>>,
    /// Remote-ID element order (same enumeration as `circuit_id`).
    pub remote_id: Option<Vec<i32>>,
}

/// SSID band steer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandSteer {
    /// Band steer mode. `0`: Disable; `1`: Prefer 5 GHz/6 GHz; `2`: Balance;
    /// `3`: Use site setting.
    pub mode: i32,
}

/// PLMN ID entry used in Hotspot 2.0 configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlmnId {
    /// Public Land Mobile Network ID (10000–999999).
    pub value: Option<String>,
}

/// Roaming Consortium OI entry used in Hotspot 2.0 configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoamingConsortiumOi {
    /// Roaming Consortium Operator Identifier in `XX-XX-XX` or
    /// `XX-XX-XX-XX-XX` hex format.
    pub value: Option<String>,
}

/// Venue information used in Hotspot 2.0 configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueInfo {
    /// Venue group (0–11, see API spec for full enumeration).
    pub group: i32,
    /// Venue type within the group (see API spec for full enumeration).
    #[serde(rename = "type")]
    pub venue_type: i32,
    /// Venue name (1–64 visible ASCII characters).
    pub name: Option<String>,
}

/// EAP authentication parameter in a NAI realm entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationParam {
    /// EAP parameter identifier (`2`, `3`, `5`, or `6`; see API spec).
    pub id: i32,
    /// EAP parameter value (see API spec for enumeration per identifier).
    pub value: i32,
}

/// EAP method in a NAI realm entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EapMethod {
    /// EAP authentication method (see API spec for enumeration).
    pub method: i32,
    /// Authentication parameter list (up to 4 entries).
    pub param: Vec<AuthenticationParam>,
}

/// NAI realm entry in Hotspot 2.0 configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Realm {
    /// NAI realm name/domain (1–64 UTF-8 characters).
    pub name: String,
    /// Encoding format. `0`: RFC4282; `1`: UTF-8.
    pub encoding: i32,
    /// EAP method list (up to 4 entries).
    pub eap: Vec<EapMethod>,
}

/// Hotspot 2.0 (802.11u) configuration for an SSID.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotspotV2Setting {
    /// Whether Hotspot 2.0 is enabled.
    pub hotspot_v2_enable: bool,
    /// 802.11u network type (0–15; see API spec for full enumeration).
    pub network_type: Option<i32>,
    /// PLMN ID list (up to 6 entries).
    pub plmn_id: Option<Vec<PlmnId>>,
    /// Roaming Consortium OI list (up to 3 entries).
    pub roaming_consortium_oi: Option<Vec<RoamingConsortiumOi>>,
    /// Hotspot operator domain name (e.g. `"www.omadanetworks.com"`).
    pub operator_domain: Option<String>,
    /// Whether DGAF (downstream group-addressed forwarding) disable mode is
    /// enabled.
    pub dgaf_disable: Option<bool>,
    /// Homogeneous ESS ID (must match one of the AP BSSIDs in the zone).
    pub he_ssid: Option<String>,
    /// Whether Internet access is indicated as available.
    pub internet: Option<bool>,
    /// IPv4 address availability type (0–7; see API spec).
    pub availability_ipv4: Option<i32>,
    /// IPv6 address availability type (0–2; see API spec).
    pub availability_ipv6: Option<i32>,
    /// Operator friendly name (1–64 visible ASCII characters).
    pub operator_friendly: Option<String>,
    /// Venue information.
    pub venue_info: Option<VenueInfo>,
    /// NAI realm list (up to 10 entries).
    pub realm_list: Option<Vec<Realm>>,
}

// ── SSID response types ───────────────────────────────────────────────────────

/// Summary of an SSID as returned in a paginated list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ssid {
    /// SSID ID.
    pub ssid_id: String,
    /// SSID name (1–32 UTF-8 characters).
    pub name: String,
    /// Band bitmask. Bit 0: 2.4 GHz; bit 1: 5 GHz; bit 2: 6 GHz.
    /// E.g. `7` (0b111) = all bands enabled.
    pub band: Option<i32>,
    /// Whether the guest network is enabled.
    pub guest_net_enable: Option<bool>,
    /// Security mode. `0`: None; `2`: WPA-Enterprise; `3`: WPA-Personal;
    /// `4`: PPSK without RADIUS; `5`: PPSK with RADIUS.
    pub security: Option<i32>,
    /// Whether SSID broadcast is enabled.
    pub broadcast: Option<bool>,
    /// Whether VLAN is enabled.
    pub vlan_enable: Option<bool>,
    /// VLAN ID (1–4094). Required when `vlan_enable` is `true`.
    pub vlan_id: Option<i32>,
    /// VLAN pool IDs string. Required when `vlan_enable` is `true`.
    pub vlan_pool_ids: Option<String>,
}

/// Full detail of an SSID, including all sub-configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsidDetail {
    /// SSID ID.
    pub ssid_id: String,
    /// SSID name (1–32 UTF-8 characters).
    pub name: String,
    /// Band bitmask. Bit 0: 2.4 GHz; bit 1: 5 GHz; bit 2: 6 GHz.
    pub band: Option<i32>,
    /// Whether auto WAN access is enabled.
    pub auto_wan_access: Option<bool>,
    /// Whether the guest network is enabled.
    pub guest_net_enable: Option<bool>,
    /// Security mode. `0`: None; `2`: WPA-Enterprise; `3`: WPA-Personal;
    /// `4`: PPSK without RADIUS; `5`: PPSK with RADIUS.
    pub security: Option<i32>,
    /// Whether Opportunistic Wireless Encryption (OWE/Enhanced Open) is enabled.
    pub owe_enable: Option<bool>,
    /// Whether SSID broadcast is enabled.
    pub broadcast: Option<bool>,
    /// Whether VLAN is enabled.
    pub vlan_enable: Option<bool>,
    /// VLAN ID (1–4094).
    pub vlan_id: Option<i32>,
    /// Whether the SSID password is hidden in responses.
    pub hide_pwd: Option<bool>,
    /// VLAN setting (alternative to plain `vlan_id`).
    pub vlan_setting: Option<SsidVlanSetting>,
    /// WPA-Personal settings.
    pub psk_setting: Option<SsidPskSetting>,
    /// WPA-Enterprise settings.
    pub ent_setting: Option<SsidEnterpriseSetting>,
    /// PPSK settings.
    pub ppsk_setting: Option<SsidPpskSetting>,
    /// Whether MLO (Multi-Link Operation) is enabled.
    pub mlo_enable: Option<bool>,
    /// PMF mode. `1`: Mandatory; `2`: Capable; `3`: Disable.
    pub pmf_mode: Option<i32>,
    /// Whether 802.11r (fast BSS transition) is enabled.
    pub enable11r: Option<bool>,
    /// Per-client rate limit configuration.
    pub client_rate_limit: Option<RateLimitSetting>,
    /// Per-SSID rate limit configuration.
    pub ssid_rate_limit: Option<RateLimitSetting>,
    /// WLAN schedule configuration.
    pub wlan_schedule: Option<SsidWlanSchedule>,
    /// 802.11 data rate control configuration.
    pub rate_control: Option<SsidRateControl>,
    /// MAC filter configuration.
    pub mac_filter: Option<SsidMacFilter>,
    /// Multicast/broadcast management configuration.
    pub multi_cast: Option<SsidMulticast>,
    /// DHCP Option 82 configuration.
    pub dhcp_option82: Option<SsidDhcpOption>,
    /// Device type bitmask. Bit 0: EAP; bit 1: Gateway.
    pub device_type: Option<i32>,
    /// Whether Wi-Fi share is prohibited.
    pub prohibit_wifi_share: Option<bool>,
    /// Hotspot 2.0 configuration.
    pub hotspot_v2_setting: Option<HotspotV2Setting>,
    /// Whether Wi-Fi Calling is enabled.
    pub wifi_calling_enable: Option<bool>,
    /// Wi-Fi Calling Profile ID.
    pub wifi_calling_id: Option<String>,
    /// Band steer configuration (returned by the API as `ssidDhcpOption`; see
    /// API spec note).
    pub ssid_dhcp_option: Option<BandSteer>,
}

// ── SSID request types ────────────────────────────────────────────────────────

/// Request body for [`OmadaClient::create_ssid`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct CreateSsidRequest {
    /// SSID name (1–32 UTF-8 characters).
    pub name: String,
    /// Device type bitmask. Bit 0: EAP; bit 1: Gateway.
    pub device_type: i32,
    /// Band bitmask. Bit 0: 2.4 GHz; bit 1: 5 GHz; bit 2: 6 GHz.
    pub band: i32,
    /// Whether the guest network is enabled.
    pub guest_net_enable: bool,
    /// Security mode. `0`: None; `2`: WPA-Enterprise; `3`: WPA-Personal;
    /// `4`: PPSK without RADIUS; `5`: PPSK with RADIUS.
    pub security: i32,
    /// Whether SSID broadcast is enabled.
    pub broadcast: bool,
    /// Whether VLAN is enabled.
    pub vlan_enable: bool,
    /// VLAN ID (1–4094). Required when `vlan_enable` is `true` and `vlan_setting`
    /// is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_id: Option<i32>,
    /// WPA-Personal settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psk_setting: Option<SsidPskSetting>,
    /// WPA-Enterprise settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent_setting: Option<SsidEnterpriseSetting>,
    /// PPSK settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppsk_setting: Option<SsidPpskSetting>,
    /// Whether MLO (Multi-Link Operation) is enabled.
    pub mlo_enable: bool,
    /// PMF mode. `1`: Mandatory; `2`: Capable; `3`: Disable.
    pub pmf_mode: i32,
    /// Whether 802.11r (fast BSS transition) is enabled.
    pub enable11r: bool,
    /// Whether to hide the SSID password in responses.
    pub hide_pwd: bool,
    /// Whether `EoGRE` tunnel is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gre_enable: Option<bool>,
    /// VLAN setting (alternative to `vlan_id`). Mutually exclusive with
    /// `vlan_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_setting: Option<SsidVlanSetting>,
    /// Whether Wi-Fi share is prohibited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prohibit_wifi_share: Option<bool>,
    /// Whether Wi-Fi Calling is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_calling_enable: Option<bool>,
    /// Wi-Fi Calling Profile ID. Required when `wifi_calling_enable` is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_calling_id: Option<String>,
}

/// Request body for [`OmadaClient::update_ssid_basic_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct UpdateSsidBasicConfigRequest {
    /// SSID name (1–32 UTF-8 characters).
    pub name: String,
    /// Band bitmask. Bit 0: 2.4 GHz; bit 1: 5 GHz; bit 2: 6 GHz.
    pub band: i32,
    /// Whether the guest network is enabled.
    pub guest_net_enable: bool,
    /// Security mode. `0`: None; `2`: WPA-Enterprise; `3`: WPA-Personal;
    /// `4`: PPSK without RADIUS; `5`: PPSK with RADIUS.
    pub security: i32,
    /// Whether SSID broadcast is enabled.
    pub broadcast: bool,
    /// Whether VLAN is enabled.
    pub vlan_enable: bool,
    /// Whether MLO (Multi-Link Operation) is enabled.
    pub mlo_enable: bool,
    /// PMF mode. `1`: Mandatory; `2`: Capable; `3`: Disable.
    pub pmf_mode: i32,
    /// Whether 802.11r (fast BSS transition) is enabled.
    pub enable11r: bool,
    /// Whether auto WAN access is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_wan_access: Option<bool>,
    /// Whether Opportunistic Wireless Encryption (OWE) is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owe_enable: Option<bool>,
    /// VLAN ID (1–4094). Required when `vlan_enable` is `true` and `vlan_setting`
    /// is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_id: Option<i32>,
    /// WPA-Personal settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psk_setting: Option<SsidPskSetting>,
    /// WPA-Enterprise settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent_setting: Option<SsidEnterpriseSetting>,
    /// PPSK settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppsk_setting: Option<SsidPpskSetting>,
    /// Whether to hide the SSID password in responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_pwd: Option<bool>,
    /// Whether `EoGRE` tunnel is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gre_enable: Option<bool>,
    /// VLAN setting (alternative to `vlan_id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_setting: Option<SsidVlanSetting>,
    /// Whether Wi-Fi share is prohibited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prohibit_wifi_share: Option<bool>,
}

/// Request body for [`OmadaClient::update_ssid_wlan_schedule`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidWlanScheduleRequest {
    /// Whether the WLAN schedule is enabled.
    pub wlan_schedule_enable: bool,
    /// Schedule action. `0`: radio off during selected period; `1`: radio on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<i32>,
    /// Time Range Profile ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_id: Option<String>,
}

/// Request body for [`OmadaClient::update_ssid_wifi_calling`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidWifiCallingRequest {
    /// Whether Wi-Fi Calling is enabled.
    pub wifi_calling_enable: bool,
    /// Wi-Fi Calling Profile ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_calling_id: Option<String>,
}

/// Request body for [`OmadaClient::update_ssid_rate_limit`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidRateLimitRequest {
    /// Per-client rate limit configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_rate_limit: Option<RateLimitSetting>,
    /// Per-SSID rate limit configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid_rate_limit: Option<RateLimitSetting>,
}

/// Request body for [`OmadaClient::update_ssid_rate_control`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidRateControlRequest {
    /// Whether 2.4 GHz data rate control is enabled.
    pub rate2g_ctrl_enable: bool,
    /// Whether 5 GHz data rate control is enabled.
    pub rate5g_ctrl_enable: bool,
    /// 2.4 GHz lower density threshold in Mbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_density2g: Option<f64>,
    /// 2.4 GHz higher density threshold in Mbps (fixed at `54`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub higher_density2g: Option<i32>,
    /// Whether to disable 2G CCK rates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cck_rates_disable: Option<bool>,
    /// Whether to require clients to use rates at or above the 2.4 GHz threshold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_rates_require2g: Option<bool>,
    /// Whether to send beacons at 1 Mbps on 2.4 GHz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_beacons2g: Option<bool>,
    /// 5 GHz lower density threshold in Mbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_density5g: Option<i32>,
    /// 5 GHz higher density threshold in Mbps (fixed at `54`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub higher_density5g: Option<i32>,
    /// Whether to require clients to use rates at or above the 5 GHz threshold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_rates_require5g: Option<bool>,
    /// Whether to send beacons at 6 Mbps on 5 GHz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_beacons5g: Option<bool>,
    /// Whether 2.4 GHz manage rate control is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manage_rate_control2g_enable: Option<bool>,
    /// 2.4 GHz manage rate control lower density value in Mbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manage_rate_control2g: Option<f64>,
    /// Whether 5 GHz manage rate control is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manage_rate_control5g_enable: Option<bool>,
    /// 5 GHz manage rate control lower density value in Mbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manage_rate_control5g: Option<i32>,
}

/// Request body for [`OmadaClient::update_ssid_multicast`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct UpdateSsidMulticastRequest {
    /// Whether multicast-to-unicast conversion is enabled.
    pub multi_cast_enable: bool,
    /// Channel utilisation threshold (0–100).
    pub channel_util: i32,
    /// Whether ARP-to-unicast conversion is enabled.
    pub arp_cast_enable: bool,
    /// Whether IPv6 multicast-to-unicast conversion is enabled.
    pub ipv6_cast_enable: bool,
    /// Whether the multicast filter is enabled.
    pub filter_enable: bool,
    /// Bitmask of enabled filter protocols. Bit 0: IGMP; bit 1: mDNS; bit 2:
    /// Others.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_mode: Option<i32>,
    /// MAC Group Profile ID for multicast filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_group_id: Option<String>,
}

/// Request body for [`OmadaClient::update_ssid_mac_filter`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidMacFilterRequest {
    /// Whether MAC filtering is enabled.
    pub mac_filter_enable: bool,
    /// Filter policy. `0`: Deny List; `1`: Allow List.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<i32>,
    /// MAC Group Profile ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_filter_id: Option<String>,
    /// OUI Profile ID list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oui_profile_id_list: Option<Vec<String>>,
}

/// Request body for [`OmadaClient::update_ssid_load_balance`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidLoadBalanceRequest {
    /// Whether load balancing is enabled.
    pub enable: bool,
}

/// Request body for [`OmadaClient::update_ssid_hotspot_v2`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidHotspotV2Request {
    /// Whether Hotspot 2.0 is enabled.
    pub hotspot_v2_enable: bool,
    /// 802.11u network type (0–15).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_type: Option<i32>,
    /// PLMN ID list (up to 6 entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plmn_id: Option<Vec<PlmnId>>,
    /// Roaming Consortium OI list (up to 3 entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roaming_consortium_oi: Option<Vec<RoamingConsortiumOi>>,
    /// Hotspot operator domain name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_domain: Option<String>,
    /// Whether DGAF disable mode is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dgaf_disable: Option<bool>,
    /// Homogeneous ESS ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub he_ssid: Option<String>,
    /// Whether Internet access is indicated as available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internet: Option<bool>,
    /// IPv4 address availability type (0–7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_ipv4: Option<i32>,
    /// IPv6 address availability type (0–2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_ipv6: Option<i32>,
    /// Operator friendly name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_friendly: Option<String>,
    /// Venue information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue_info: Option<VenueInfo>,
    /// NAI realm list (up to 10 entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm_list: Option<Vec<Realm>>,
}

/// Request body for [`OmadaClient::update_ssid_dhcp_option`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidDhcpOptionRequest {
    /// Whether DHCP Option 82 is enabled.
    pub dhcp_enable: bool,
    /// Option 82 encoding format. `0`: ASCII; `1`: Binary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<i32>,
    /// Option 82 delimiter (a single printable ASCII character).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Circuit-ID element order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_id: Option<Vec<i32>>,
    /// Remote-ID element order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_id: Option<Vec<i32>>,
}

/// Request body for [`OmadaClient::update_ssid_band_steer`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSsidBandSteerRequest {
    /// Band steer mode. `0`: Disable; `1`: Prefer 5 GHz/6 GHz; `2`: Balance;
    /// `3`: Use site setting.
    pub mode: i32,
}

// ── Access Point (AP) types ───────────────────────────────────────────────────

/// Overview information for an AP device, returned by [`OmadaClient::ap`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApOverview {
    /// Device type string.
    pub r#type: Option<String>,
    /// Device MAC address, e.g. `AA-BB-CC-DD-EE-FF`.
    pub mac: Option<String>,
    /// Device name.
    pub name: Option<String>,
    /// Device IP address.
    pub ip: Option<String>,
    /// Device IPv6 addresses.
    pub ipv6_list: Option<Vec<String>>,
    /// WLAN group ID.
    #[serde(rename = "wlan group id")]
    pub wlan_group_id: Option<String>,
    /// Wireless uplink information. Present when the AP has a wireless uplink.
    #[serde(rename = "wireless uplink info")]
    pub wireless_uplink_info: Option<ApWirelessUplink>,
    /// Device model identifier.
    pub model: Option<String>,
    /// Device firmware version string.
    pub firmware_version: Option<String>,
    /// CPU utilization percentage (e.g. `1` = 1%).
    pub cpu_util: Option<i32>,
    /// Memory utilization percentage (e.g. `50` = 50%).
    pub memory_util: Option<i32>,
    /// Device uptime in seconds.
    pub uptime_long: Option<i64>,
}

/// Wireless mesh uplink information for an AP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApWirelessUplink {
    /// MAC address of the uplink AP.
    pub uplink_mac: Option<String>,
    /// Name of the uplink AP.
    pub name: Option<String>,
    /// Channel of the uplink AP.
    pub channel: Option<i32>,
    /// RSSI to the uplink AP.
    pub rssi: Option<i32>,
    /// Signal-to-noise ratio to the uplink AP.
    pub snr: Option<i32>,
    /// Transmit rate string.
    pub tx_rate: Option<String>,
    /// Transmit rate in Mbps.
    pub tx_rate_int: Option<i32>,
    /// Receive rate string.
    pub rx_rate: Option<String>,
    /// Receive rate in Mbps.
    pub rx_rate_int: Option<i32>,
    /// Total bytes sent to the uplink AP (bytes).
    pub up_bytes: Option<i64>,
    /// Total bytes received from the uplink AP (bytes).
    pub down_bytes: Option<i64>,
    /// Total packets sent.
    pub up_packets: Option<i64>,
    /// Total packets received.
    pub down_packets: Option<i64>,
    /// Activity metric: change in (`down_bytes` + `up_bytes`) per second.
    pub activity: Option<i64>,
    /// Whether speed measurement is supported for this uplink.
    pub support_speed_test: Option<bool>,
    /// Uplink AP model.
    pub model: Option<String>,
    /// Uplink AP model version.
    pub model_version: Option<String>,
    /// Uplink AP IP address.
    pub ip: Option<String>,
    /// Uplink AP device type.
    pub r#type: Option<String>,
    /// Port on the current device used for the uplink.
    pub uplink_port: Option<String>,
}

/// Wired uplink status for an AP, returned by [`OmadaClient::ap_wired_uplink`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApWiredUplinkStatus {
    /// Wired uplink connection detail.
    pub wired_uplink: Option<ApWiredUplinkDetail>,
}

/// Detailed wired uplink information for an AP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApWiredUplinkDetail {
    /// MAC address of the uplink device.
    pub uplink_mac: Option<String>,
    /// Stack unit ID of the uplink device.
    pub stack_id: Option<String>,
    /// Device type of the uplink device.
    pub r#type: Option<String>,
    /// Name of the uplink device.
    pub name: Option<String>,
    /// Negotiation rate in Mbps; `"0"` when disconnected.
    pub rate: Option<String>,
    /// Duplex mode. `0`: disconnected; `1`: half; `2`: full.
    pub duplex: Option<i32>,
    /// Total bytes sent (bytes).
    pub up_bytes: Option<i64>,
    /// Total bytes received (bytes).
    pub down_bytes: Option<i64>,
    /// Total packets sent.
    pub up_packets: Option<i64>,
    /// Total packets received.
    pub down_packets: Option<i64>,
    /// Dropped uplink packets.
    pub up_drop_packets: Option<i64>,
    /// Dropped downlink packets.
    pub down_drop_packets: Option<i64>,
    /// Uplink error packets.
    pub up_errors_packets: Option<i64>,
    /// Downlink error packets.
    pub down_errors_packets: Option<i64>,
}

/// Wired downlink status for an AP, returned by [`OmadaClient::ap_wired_downlink`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApWiredDownlinkStatus {
    /// List of wired downlink devices.
    pub wired_downlink_list: Option<Vec<ApDownlinkStatus>>,
}

/// A single wired downlink device connected to an AP port.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApDownlinkStatus {
    /// Port identifier.
    pub port: Option<String>,
    /// Port type.
    pub port_type: Option<i32>,
    /// Duplex mode.
    pub duplex: Option<i32>,
    /// Link status.
    pub link: Option<i32>,
    /// Link speed.
    pub speed: Option<i32>,
    /// `PoE` state.
    pub poe_state: Option<i32>,
    /// `VoIP` state.
    pub voip_state: Option<i32>,
    /// Device MAC address.
    pub mac: Option<String>,
    /// Device IP address.
    pub ip: Option<String>,
    /// Device type.
    pub r#type: Option<String>,
    /// Device name.
    pub device_name: Option<String>,
    /// Device model.
    pub model: Option<String>,
    /// Device model version.
    pub model_version: Option<String>,
}

/// General configuration of an AP, returned by [`OmadaClient::ap_general_config`]
/// and used in [`OmadaClient::update_ap_general_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApGeneralConfig {
    /// Device name (1–128 characters; may not start/end with spaces or begin
    /// with `+`, `-`, `@`, `=`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// LED setting. `0`: off; `1`: on; `2`: use site settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_setting: Option<i32>,
    /// Whether the remote reset function is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_reset: Option<bool>,
    /// Whether the Wi-Fi Control function is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_control_enable: Option<bool>,
    /// RSSI LED settings list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi_led_settings: Option<Vec<ApRssiLedSetting>>,
    /// Tag IDs assigned to the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_ids: Option<Vec<String>>,
    /// Whether GPS is enabled. Only applicable to APs with GPS hardware.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps_enable: Option<bool>,
    /// Device location information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DeviceLocation>,
    /// Remember device setting. `0`: off; `1`: on; `2`: use site settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember_device: Option<i32>,
    /// When `true`, the device's hardware reset button is disabled. Only
    /// supported on specific devices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_hw_reset: Option<bool>,
}

/// Per-LED RSSI threshold setting used in [`ApGeneralConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRssiLedSetting {
    /// LED identifier name.
    pub led_id: Option<String>,
    /// RSSI threshold value.
    pub threshold: Option<i32>,
}

/// Physical location of a device, used in [`ApGeneralConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLocation {
    /// Longitude of the device.
    pub longitude: Option<f64>,
    /// Latitude of the device.
    pub latitude: Option<f64>,
    /// Street address of the device.
    pub address: Option<String>,
    /// Timestamp of the last GPS fix (Unix seconds).
    pub timestamp: Option<i64>,
}

/// IP settings for an AP, returned by [`OmadaClient::ap_ip_setting`] and used
/// in [`OmadaClient::update_ap_ip_setting`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApIpSetting {
    /// Address assignment mode: `"static"` or `"dhcp"`.
    pub mode: String,
    /// DHCP mode IP settings. Present when `mode` is `"dhcp"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhcp_ip_setting: Option<DhcpIpSetting>,
    /// Static mode IP settings. Present when `mode` is `"static"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_ip_setting: Option<StaticIpSetting>,
}

/// DHCP IP address settings for an AP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpIpSetting {
    /// Whether to use a static fallback address when DHCP is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<bool>,
    /// Fallback IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_ip: Option<String>,
    /// Fallback subnet mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_mask: Option<String>,
    /// Fallback gateway IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_gate: Option<String>,
    /// Whether to request a specific reserved address (requires gateway).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_fixed_addr: Option<bool>,
    /// LAN network ID for the reserved address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_id: Option<String>,
    /// DHCP-assigned IP address (read-only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhcp_ip: Option<String>,
}

/// Static IP address settings for an AP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticIpSetting {
    /// Static IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_ip: Option<String>,
    /// Subnet mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_mask: Option<String>,
    /// Default gateway IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_gate: Option<String>,
    /// Primary DNS server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_dns: Option<String>,
    /// Secondary DNS server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternate_dns: Option<String>,
}

/// IPv6 settings for an AP, returned by [`OmadaClient::ap_ipv6_setting`] and
/// used in [`OmadaClient::update_ap_ipv6_setting`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApIpv6Setting {
    /// Whether IPv6 is enabled.
    pub ipv6_enable: bool,
    /// Address assignment mode: `"static"` or `"dynamic"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Dynamic IPv6 settings. Present when `mode` is `"dynamic"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_ipv6_setting: Option<DynamicIpv6Setting>,
    /// Static IPv6 settings. Present when `mode` is `"static"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_ipv6_setting: Option<StaticIpv6Setting>,
}

/// Dynamic IPv6 settings for an AP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicIpv6Setting {
    /// DNS resolution mode. `0`: get from DHCP; `1`: use custom DNS addresses.
    pub dns_mode: i32,
    /// Primary DNS address (valid IPv6). Required when `dns_mode` is `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pri_dns: Option<String>,
    /// Secondary DNS address (valid IPv6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sec_dns: Option<String>,
}

/// Static IPv6 settings for an AP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticIpv6Setting {
    /// Static IPv6 address.
    pub ipv6_addr: String,
    /// IPv6 prefix length (1–128).
    pub prefix_len: i32,
    /// IPv6 gateway address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    /// Primary DNS server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pri_dns: Option<String>,
    /// Secondary DNS server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sec_dns: Option<String>,
}

/// Radio configuration for an AP, returned by [`OmadaClient::ap_radio_config`]
/// and used in [`OmadaClient::update_ap_radio_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRadiosConfig {
    /// 2.4 GHz radio settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_setting2g: Option<ApRadioSetting>,
    /// 5 GHz radio settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_setting5g: Option<ApRadioSetting>,
    /// 5 GHz-1 radio settings (on devices with frequency splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_setting5g1: Option<ApRadioSetting>,
    /// 5 GHz-2 radio settings (on devices with frequency splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_setting5g2: Option<ApRadioSetting>,
    /// 6 GHz radio settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_setting6g: Option<ApRadioSetting>,
}

/// Settings for a single radio band on an AP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRadioSetting {
    /// Whether this radio band is enabled. When `false`, all other fields are
    /// ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_enable: Option<bool>,
    /// Custom optional channel frequency list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_range: Option<Vec<i32>>,
    /// Channel width code (e.g. `2`=20 MHz, `3`=40 MHz, `5`=80 MHz, `7`=160 MHz).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_width: Option<String>,
    /// Channel index. `"0"` selects auto. Obtain valid values from the
    /// available channel list endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Transmit power in dBm. Used when `tx_power_level` is `3` (Custom).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_power: Option<i32>,
    /// Transmit power level preset. `0`: Low; `1`: Medium; `2`: High;
    /// `3`: Custom; `4`: Auto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_power_level: Option<i32>,
    /// Whether site-level channel limit is enabled for this band.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_limit_enable: Option<bool>,
    /// Channel frequency. Should correspond to `channel`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freq: Option<i32>,
    /// Wireless mode. `-2`: Auto; others: various 802.11 mixed modes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wireless_mode: Option<i32>,
}

/// Current radio status and traffic counters for an AP, returned by
/// [`OmadaClient::ap_radios`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRadiosDetail {
    /// 2.4 GHz radio traffic counters.
    pub radio_traffic2g: Option<ApRadioTraffic>,
    /// 5 GHz radio traffic counters.
    pub radio_traffic5g: Option<ApRadioTraffic>,
    /// 5 GHz-2 radio traffic counters.
    pub radio_traffic5g2: Option<ApRadioTraffic>,
    /// 6 GHz radio traffic counters.
    pub radio_traffic6g: Option<ApRadioTraffic>,
    /// 2.4 GHz channel detail.
    pub wp2g: Option<ApRadioChannel>,
    /// 5 GHz channel detail.
    pub wp5g: Option<ApRadioChannel>,
    /// 5 GHz-2 channel detail.
    pub wp5g2: Option<ApRadioChannel>,
    /// 6 GHz channel detail.
    pub wp6g: Option<ApRadioChannel>,
}

/// Per-band channel information for an AP radio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRadioChannel {
    /// Currently active channel.
    pub actual_channel: Option<String>,
    /// Maximum transmit rate (Mbps).
    pub max_tx_rate: Option<i32>,
    /// Transmit power (dBm).
    pub tx_power: Option<i32>,
    /// Regulatory region code.
    pub region: Option<i32>,
    /// Channel bandwidth string.
    pub band_width: Option<String>,
    /// Regulatory domain mode.
    pub rd_mode: Option<String>,
    /// Transmit channel utilization (0–100).
    pub tx_util: Option<i32>,
    /// Receive channel utilization (0–100).
    pub rx_util: Option<i32>,
    /// Interference channel utilization (0–100).
    pub inter_util: Option<i32>,
}

/// Radio traffic counters for a single band.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRadioTraffic {
    /// Total received packets.
    pub rx_pkts: Option<i64>,
    /// Total transmitted packets.
    pub tx_pkts: Option<i64>,
    /// Total received bytes.
    pub rx: Option<i64>,
    /// Total transmitted bytes.
    pub tx: Option<i64>,
    /// Received dropped packets.
    pub rx_drop_pkts: Option<i64>,
    /// Transmitted dropped packets.
    pub tx_drop_pkts: Option<i64>,
    /// Received error packets.
    pub rx_err_pkts: Option<i64>,
    /// Transmitted error packets.
    pub tx_err_pkts: Option<i64>,
    /// Received retry packets.
    pub rx_retry_pkts: Option<i64>,
}

/// OFDMA configuration for an AP, returned by [`OmadaClient::ap_ofdma_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApOfdmaConfig {
    /// Whether OFDMA is enabled on 2.4 GHz.
    pub ofdma_enable2g: Option<bool>,
    /// Whether OFDMA is enabled on 5 GHz.
    pub ofdma_enable5g: Option<bool>,
    /// Whether OFDMA is enabled on 5 GHz-2.
    pub ofdma_enable5g2: Option<bool>,
    /// Whether OFDMA is enabled on 6 GHz.
    pub ofdma_enable6g: Option<bool>,
    /// Whether the device supports OFDMA on 2.4 GHz.
    pub support_ofdma2g: Option<bool>,
    /// Whether the device supports OFDMA on 5 GHz.
    pub support_ofdma5g: Option<bool>,
    /// Whether the device supports OFDMA on 5 GHz-2.
    pub support_ofdma5g2: Option<bool>,
    /// Whether the device supports OFDMA on 6 GHz.
    pub support_ofdma6g: Option<bool>,
}

/// Request body for [`OmadaClient::update_ap_ofdma_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApOfdmaConfig {
    /// Enable/disable OFDMA on 2.4 GHz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ofdma_enable2g: Option<bool>,
    /// Enable/disable OFDMA on 5 GHz (whole band, for devices without
    /// frequency splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ofdma_enable5g: Option<bool>,
    /// Enable/disable OFDMA on 5 GHz-2 (for devices with frequency splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ofdma_enable5g2: Option<bool>,
    /// Enable/disable OFDMA on 6 GHz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ofdma_enable6g: Option<bool>,
}

/// `QoS` (Unscheduled Automatic Power Save Delivery) configuration for an AP,
/// returned by [`OmadaClient::ap_qos_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApQosConfig {
    /// Whether the device supports 2.4 GHz.
    pub support2g: Option<bool>,
    /// Whether the device supports 5 GHz.
    pub support5g: Option<bool>,
    /// Whether the device supports 5 GHz frequency splitting.
    pub support5g2: Option<bool>,
    /// Whether the device supports 6 GHz.
    pub support6g: Option<bool>,
    /// Whether U-APSD is enabled on 2.4 GHz.
    pub delivery_enable2g: Option<bool>,
    /// Whether U-APSD is enabled on 5 GHz (or 5 GHz-1 when splitting is
    /// active).
    pub delivery_enable5g: Option<bool>,
    /// Whether U-APSD is enabled on 5 GHz-2. Present when 5 GHz splitting is
    /// supported.
    pub delivery_enable5g2: Option<bool>,
    /// Whether U-APSD is enabled on 6 GHz.
    pub delivery_enable6g: Option<bool>,
}

/// Request body for [`OmadaClient::update_ap_qos_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApQosConfig {
    /// Enable/disable U-APSD on 2.4 GHz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_enable2g: Option<bool>,
    /// Enable/disable U-APSD on the entire 5 GHz band (for devices without
    /// frequency splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_enable5g: Option<bool>,
    /// Enable/disable U-APSD on 5 GHz-1 (for devices with frequency
    /// splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_enable5g1: Option<bool>,
    /// Enable/disable U-APSD on 5 GHz-2 (for devices with frequency
    /// splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_enable5g2: Option<bool>,
    /// Enable/disable U-APSD on 6 GHz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_enable6g: Option<bool>,
}

/// Per-band load-balance configuration for an AP, returned by
/// [`OmadaClient::ap_load_balance_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApLoadBalanceConfig {
    /// Whether the 2.4 GHz maximum client limit is enabled.
    pub max_clients_enable2g: Option<bool>,
    /// Maximum associated clients on 2.4 GHz (1–512).
    pub max_clients2g: Option<i32>,
    /// Whether the 2.4 GHz RSSI threshold is enabled.
    pub rssi_enable2g: Option<bool>,
    /// RSSI threshold for 2.4 GHz (-95–0 dBm).
    pub threshold2g: Option<i32>,
    /// Whether the 5 GHz maximum client limit is enabled.
    pub max_clients_enable5g: Option<bool>,
    /// Maximum associated clients on 5 GHz (1–512).
    pub max_clients5g: Option<i32>,
    /// Whether the 5 GHz RSSI threshold is enabled.
    pub rssi_enable5g: Option<bool>,
    /// RSSI threshold for 5 GHz (-95–0 dBm).
    pub threshold5g: Option<i32>,
    /// Whether the 5 GHz-2 maximum client limit is enabled.
    pub max_clients_enable5g2: Option<bool>,
    /// Maximum associated clients on 5 GHz-2 (1–512).
    pub max_clients5g2: Option<i32>,
    /// Whether the 5 GHz-2 RSSI threshold is enabled.
    pub rssi_enable5g2: Option<bool>,
    /// RSSI threshold for 5 GHz-2 (-95–0 dBm).
    pub threshold5g2: Option<i32>,
    /// Whether the 6 GHz maximum client limit is enabled.
    pub max_clients_enable6g: Option<bool>,
    /// Maximum associated clients on 6 GHz (1–512).
    pub max_clients6g: Option<i32>,
    /// Whether the 6 GHz RSSI threshold is enabled.
    pub rssi_enable6g: Option<bool>,
    /// RSSI threshold for 6 GHz (-95–0 dBm).
    pub threshold6g: Option<i32>,
}

/// Request body for [`OmadaClient::update_ap_load_balance_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApLoadBalanceConfig {
    /// Enable/disable the 2.4 GHz maximum client limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients_enable2g: Option<bool>,
    /// Maximum associated clients on 2.4 GHz (1–512).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients2g: Option<i32>,
    /// Enable/disable the 2.4 GHz RSSI threshold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi_enable2g: Option<bool>,
    /// RSSI threshold for 2.4 GHz (-95–0 dBm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold2g: Option<i32>,
    /// Enable/disable the 5 GHz maximum client limit (whole band, no
    /// frequency splitting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients_enable5g: Option<bool>,
    /// Maximum associated clients on 5 GHz (whole band, 1–512).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients5g: Option<i32>,
    /// Enable/disable the 5 GHz RSSI threshold (whole band).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi_enable5g: Option<bool>,
    /// RSSI threshold for 5 GHz, whole band (-95–0 dBm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold5g: Option<i32>,
    /// Enable/disable load balance for 5 GHz-1 (frequency splitting devices).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients_enable5g1: Option<bool>,
    /// Maximum associated clients on 5 GHz-1 (1–512).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients5g1: Option<i32>,
    /// Enable/disable load balance for 5 GHz-2 (frequency splitting devices).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients_enable5g2: Option<bool>,
    /// Maximum associated clients on 5 GHz-2 (1–512).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients5g2: Option<i32>,
    /// Enable/disable the 6 GHz maximum client limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients_enable6g: Option<bool>,
    /// Maximum associated clients on 6 GHz (1–512).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clients6g: Option<i32>,
}

/// Trunk setting configuration for an AP, returned by
/// [`OmadaClient::ap_trunk_setting`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApTrunkSetting {
    /// Whether the device supports the trunk setting feature.
    pub support_trunk_setting: Option<bool>,
    /// Whether trunk setting is currently enabled.
    pub enable: Option<bool>,
    /// Trunk mode. `0`: SRC MAC + DST MAC; `1`: DST MAC; `2`: SRC MAC.
    pub mode: Option<i32>,
}

/// Request body for [`OmadaClient::update_ap_trunk_setting`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApTrunkSetting {
    /// Whether to enable trunk setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// Trunk mode. `0`: SRC MAC + DST MAC; `1`: DST MAC; `2`: SRC MAC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<i32>,
}

/// Request body for [`OmadaClient::update_ap_service_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApServicesConfig {
    /// Management VLAN setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvlan_setting: Option<ApMvlanSetting>,
    /// SNMP location/contact setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snmp: Option<ApSnmpInfo>,
    /// EAP L3 accessibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l3_access_setting: Option<ApL3Access>,
    /// LLDP enable state. `0`: off; `1`: on; `2`: use site settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lldp_enable: Option<i32>,
    /// Whether loopback detection is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loopback_detect_enable: Option<bool>,
    /// `VoIP` VLAN setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voip_vlan_setting: Option<ApVoipVlanSetting>,
}

/// Management VLAN setting used in [`ApServicesConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMvlanSetting {
    /// Management VLAN mode. `0`: Default; `1`: Custom.
    pub mode: i32,
    /// LAN network ID for the effective management VLAN. Used when `mode` is
    /// `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lan_network_id: Option<String>,
    /// Bridge VLAN ID. Set when the management VLAN network uses a bridge
    /// VLAN.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_vlan: Option<i32>,
}

/// SNMP location and contact setting used in [`ApServicesConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApSnmpInfo {
    /// SNMP location string (0–128 ASCII characters; no leading/trailing
    /// spaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// SNMP contact string (0–128 ASCII characters; no spaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

/// EAP L3 Accessibility setting used in [`ApServicesConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApL3Access {
    /// Whether EAP L3 Accessibility is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
}

/// `VoIP` VLAN setting used in [`ApServicesConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApVoipVlanSetting {
    /// `VoIP` VLAN mode. `0`: Follow Management VLAN; `1`: Custom.
    pub mode: i32,
    /// LAN network ID. Required when `mode` is `1`; must differ from the
    /// management VLAN network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lan_network_id: Option<String>,
    /// `VoIP` bridge VLAN ID (1–4090). Required when the LAN network uses
    /// multiple VLANs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voip_bridge_vlan: Option<i32>,
    /// `VoIP` VLAN IP type. `0`: Static IP; `1`: DHCP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_type: Option<i32>,
    /// Static IP address. Required when `ip_type` is `0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

/// P2P bridge configuration for an AP, returned by
/// [`OmadaClient::ap_bridge_config`] and used in
/// [`OmadaClient::update_ap_bridge_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApBridgeConfig {
    /// Bridge SSID name (1–32 UTF-8 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_ssid_name: Option<String>,
    /// Bridge SSID password (8–63 printable ASCII characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_ssid_password: Option<String>,
    /// Bridge DIP switch state. `0`: disable; `1`: enable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hw_switch: Option<i32>,
    /// TDMA configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tdma_config: Option<ApBridgeTdmaConfig>,
}

/// TDMA configuration within a P2P bridge, used in [`ApBridgeConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApBridgeTdmaConfig {
    /// TDMA status. `0`: disable; `1`: enable.
    pub status: i32,
    /// TDMA client AP entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clients: Option<Vec<BridgeClientApConfig>>,
}

/// A TDMA client AP entry within a bridge configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeClientApConfig {
    /// MAC address of the client AP.
    pub mac: String,
    /// Priority. `0`: high; `1`: base; `2`: low.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Antenna gain configuration for an AP, returned by
/// [`OmadaClient::ap_antenna_gain`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApAntennaGainConfig {
    /// Per-radio antenna gain entries.
    pub ante_gain_list: Option<Vec<ApRadioAntennaGain>>,
}

/// Antenna gain for a single radio band, used in [`ApAntennaGainConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRadioAntennaGain {
    /// Radio ID. `0`: 2.4 GHz; `1`: 5 GHz; `2`: 5 GHz-2; `3`: 6 GHz.
    pub radio_id: Option<i32>,
    /// Antenna gain in dBi (0–30).
    pub ante_gain: Option<i32>,
}

/// Request body for [`OmadaClient::update_ap_antenna_gain`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApAntennaGainConfig {
    /// Per-radio antenna gain updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ante_gain_list: Option<Vec<UpdateApRadioAntennaGain>>,
}

/// Antenna gain update entry for a single radio band.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApRadioAntennaGain {
    /// Radio ID. `0`: 2.4 GHz; `1`: 5 GHz; `2`: 5 GHz-2; `3`: 6 GHz.
    pub radio_id: i32,
    /// Antenna gain in dBi (0–30).
    pub ante_gain: i32,
}

/// Antenna switch configuration for an AP, returned by
/// [`OmadaClient::ap_ant_switch_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApAntSwitchConfig {
    /// Antenna mode. `0`: Auto; `1`: Built-in; `2`: Omni; `3`: Custom.
    pub ant_mode: Option<i32>,
    /// Omni antenna install state. `0`: not installed; `1`: abnormal;
    /// `2`: normal.
    pub omni_ant_state: Option<i32>,
    /// 2.4 GHz antenna setting.
    pub ant_setting2g: Option<AntSetting>,
    /// 5 GHz antenna setting.
    pub ant_setting5g: Option<AntSetting>,
    /// 5 GHz-2 antenna setting.
    pub ant_setting5g2: Option<AntSetting>,
    /// 6 GHz antenna setting.
    pub ant_setting6g: Option<AntSetting>,
}

/// Per-band antenna setting as reported by the device.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntSetting {
    /// Antenna mode for this band.
    pub ant_mode: Option<i32>,
    /// Antenna gain in dBi.
    pub ant_gain: Option<i32>,
    /// Antenna pattern. `0`: built-in; `1`: external.
    pub ant_pattern: Option<i32>,
    /// Maximum gain limit for custom mode.
    pub custom_gain_limit: Option<i32>,
    /// Default Omni antenna mode gain.
    pub default_omni_gain: Option<i32>,
}

/// Request body for [`OmadaClient::update_ap_ant_switch_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApAntSwitchConfig {
    /// Antenna mode. `0`: Auto; `1`: Built-in; `2`: Omni; `3`: Custom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_mode: Option<i32>,
    /// 2.4 GHz antenna radio config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_setting2g: Option<AntSwitchRadioConfig>,
    /// 5 GHz antenna radio config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_setting5g: Option<AntSwitchRadioConfig>,
    /// 5 GHz-2 antenna radio config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_setting5g2: Option<AntSwitchRadioConfig>,
    /// 6 GHz antenna radio config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_setting6g: Option<AntSwitchRadioConfig>,
}

/// Per-band antenna configuration for an update request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntSwitchRadioConfig {
    /// Antenna mode. `0`: Auto; `1`: Built-in; `2`: Omni.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_mode: Option<i32>,
    /// Antenna gain in dBi.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ant_gain: Option<i32>,
}

/// AFC (Automated Frequency Coordination) configuration for an AP, returned
/// by [`OmadaClient::ap_afc_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApAfcConfig {
    /// Whether the device supports AFC.
    pub support_afc: Option<bool>,
    /// Whether AFC is currently enabled.
    pub enable: Option<bool>,
}

/// Request body for [`OmadaClient::update_ap_afc_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApAfcConfig {
    /// Whether to enable AFC.
    pub enable: bool,
}

/// AP uplink port configuration, returned by [`OmadaClient::ap_uplink_config`]
/// and used in [`OmadaClient::update_ap_uplink_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApUplinkConfig {
    /// Whether the uplink port is selected automatically.
    pub auto: bool,
    /// Port identifier. Required when `auto` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
}

/// Power saving configuration for an AP, returned by
/// [`OmadaClient::ap_power_saving_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPowerSavingConfig {
    /// Whether time-triggered power saving is enabled.
    pub time_enable: Option<bool>,
    /// Start hour for the time trigger (0–23).
    pub start_time_h: Option<i32>,
    /// Start minute for the time trigger (0–59).
    pub start_time_m: Option<i32>,
    /// End hour for the time trigger (0–23).
    pub end_time_h: Option<i32>,
    /// End minute for the time trigger (0–59).
    pub end_time_m: Option<i32>,
    /// Whether band-triggered power saving is enabled.
    pub band_enable: Option<bool>,
    /// Selected bands for the band trigger. `0`: 2.4 GHz; `1`: 5 GHz;
    /// `2`: 5 GHz-2; `3`: 6 GHz.
    pub bands: Option<Vec<i32>>,
    /// Idle duration threshold for the band trigger.
    pub idle_duration: Option<i32>,
    /// Whether the device supports the power saving feature.
    pub support_power_saving: Option<bool>,
}

/// Request body for [`OmadaClient::update_ap_power_saving_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApPowerSavingConfig {
    /// Whether to enable time-triggered power saving.
    pub time_enable: bool,
    /// Start hour for the time trigger (0–23). Required when `time_enable` is
    /// `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_h: Option<i32>,
    /// Start minute for the time trigger (0–59). Required when `time_enable`
    /// is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_m: Option<i32>,
    /// End hour for the time trigger (0–23). Required when `time_enable` is
    /// `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_h: Option<i32>,
    /// End minute for the time trigger (0–59). Required when `time_enable` is
    /// `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_m: Option<i32>,
    /// Whether to enable band-triggered power saving.
    pub band_enable: bool,
    /// Selected bands for the band trigger (at least one required when
    /// `band_enable` is `true`). `0`: 2.4 GHz; `1`: 5 GHz; `2`: 5 GHz-2;
    /// `3`: 6 GHz. Must be bands supported by the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bands: Option<Vec<i32>>,
    /// Idle duration for the band trigger.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_duration: Option<i32>,
}

/// Channel limit configuration for an AP, returned by
/// [`OmadaClient::ap_channel_limit_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApChannelLimitConfig {
    /// Whether the device supports the channel limit feature.
    pub support_channel_limit: Option<bool>,
    /// Channel limit enable status. `0`: default; `1`: disabled; `2`: enabled.
    pub channel_limit_type: Option<i32>,
    /// Default installation type for 5 GHz. `true`: outdoor; `false`: indoor.
    pub default_inst_type5g: Option<bool>,
    /// Default installation type for 6 GHz. `true`: outdoor; `false`: indoor.
    pub default_inst_type6g: Option<bool>,
}

/// Request body for [`OmadaClient::update_ap_channel_limit_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApChannelLimitConfig {
    /// Channel limit enable status. `0`: default; `1`: disabled; `2`: enabled.
    pub channel_limit_type: i32,
}

/// Request body for [`OmadaClient::update_ap_channel_config`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApChannelConfig {
    /// Radio ID. `0`: 2.4 GHz; `1`: 5 GHz (or 5 GHz-1); `2`: 5 GHz-2;
    /// `3`: 6 GHz.
    pub radio_id: i32,
    /// Whether this radio is enabled. When `false`, other fields are ignored.
    pub radio_enable: bool,
    /// Channel index. `0` for auto. Obtain valid values from the available
    /// channel list endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<i32>,
    /// Channel width code. `0` for auto. Obtain valid values from the
    /// available channel list endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_width: Option<i32>,
}

/// Management WLAN (SSID) configuration for an AP, returned by
/// [`OmadaClient::ap_management_wlan`] and used in
/// [`OmadaClient::update_ap_management_wlan`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApManagementWlan {
    /// Whether this management SSID is enabled.
    pub status: bool,
    /// SSID name (1–32 UTF-8 characters).
    pub name: String,
    /// Security mode. `0`: None; `2`: WPA-Enterprise; `3`: WPA-Personal.
    pub security: i32,
    /// Whether this SSID broadcasts its name.
    pub broadcast: bool,
    /// WPA-Personal settings. Required when `security` is `3`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psk_setting: Option<ApMgtPskSetting>,
    /// WPA-Enterprise settings. Required when `security` is `2`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent_setting: Option<ApMgtEnterpriseSetting>,
    /// VLAN configuration for the management SSID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_setting: Option<ApMgtVlanSetting>,
}

/// WPA-Personal settings for a management WLAN.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMgtPskSetting {
    /// WPA-Personal password (8–63 printable ASCII or hex characters).
    pub security_key: String,
    /// WPA version. `1`: WPA-PSK; `2`: WPA2-PSK; `3`: WPA/WPA2-PSK.
    pub version_psk: i32,
    /// Encryption type. `1`: Auto; `3`: AES.
    pub encryption_psk: i32,
}

/// WPA-Enterprise settings for a management WLAN.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMgtEnterpriseSetting {
    /// RADIUS profile ID.
    pub radius_profile_id: String,
    /// WPA-Enterprise version. `1`: WPA-Enterprise; `2`: WPA2-Enterprise;
    /// `3`: WPA/WPA2-Enterprise.
    pub version_ent: i32,
    /// Encryption type. `1`: Auto; `3`: AES.
    pub encryption_ent: i32,
}

/// VLAN settings for a management WLAN.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMgtVlanSetting {
    /// VLAN mode. `0`: Default; `1`: Custom.
    pub mode: i32,
    /// Custom VLAN configuration. Required when `mode` is `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_config: Option<ApMgtVlanCustomConfig>,
}

/// Custom VLAN configuration within a management WLAN VLAN setting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMgtVlanCustomConfig {
    /// Custom mode. `0`: by network; `1`: by VLAN.
    pub custom_mode: i32,
    /// LAN network ID. Required when `custom_mode` is `0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lan_network_id: Option<String>,
    /// Bridge VLAN ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_vlan: Option<i32>,
    /// VLAN ID. Required when `custom_mode` is `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_id: Option<i32>,
}

/// Management VLAN configuration for an AP, returned by
/// [`OmadaClient::ap_vlan_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApVlanConfig {
    /// Management VLAN mode. `0`: Default; `1`: Custom.
    pub mode: i32,
    /// Currently effective LAN network ID.
    pub lan_network_id: Option<String>,
    /// Bridge VLAN ID. Set when the network uses a bridge VLAN.
    pub bridge_vlan: Option<i32>,
}

/// SNMP configuration for an AP, returned by [`OmadaClient::ap_snmp_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApSnmpConfig {
    /// SNMP location string (0–128 ASCII characters; no leading/trailing
    /// spaces).
    pub location: Option<String>,
    /// SNMP contact string (0–128 ASCII characters; no spaces).
    pub contact: Option<String>,
}

/// Speed test results for an AP, returned by
/// [`OmadaClient::ap_speed_test_result`]. Only applicable to P2P devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApSpeedTestResults {
    /// Whether the main AP is currently measuring speed.
    pub main_testing: Option<bool>,
    /// MAC address of the AP that initiated the speed test.
    pub source_ap: Option<String>,
    /// MAC address of the AP under test.
    pub target_ap: Option<String>,
    /// Per-device speed measurement results keyed by AP MAC address.
    pub speed_test_result: Option<crate::JsonValue>,
}

/// RF scan results for an AP (deprecated — prefer the v2 endpoint), returned
/// by [`OmadaClient::ap_rf_scan_result`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApRfScanResult {
    /// Currently active 2.4 GHz channel.
    pub current_chan2g: Option<String>,
    /// Currently active 5 GHz channel.
    pub current_chan5g: Option<String>,
    /// Currently active 5 GHz-2 channel.
    pub current_chan5g2: Option<String>,
    /// Currently active 6 GHz channel.
    pub current_chan6g: Option<String>,
    /// 2.4 GHz channel bandwidth code.
    pub current_chan_w2g: Option<i32>,
    /// 5 GHz channel bandwidth code.
    pub current_chan_w5g: Option<i32>,
    /// 5 GHz-2 channel bandwidth code.
    pub current_chan_w5g2: Option<i32>,
    /// 6 GHz channel bandwidth code.
    pub current_chan_w6g: Option<i32>,
    /// 2.4 GHz channel scan data.
    pub channel2g: Option<crate::JsonValue>,
    /// 5 GHz channel scan data.
    pub channel5g: Option<crate::JsonValue>,
    /// 5 GHz-2 channel scan data.
    pub channel5g2: Option<crate::JsonValue>,
    /// 6 GHz channel scan data.
    pub channel6g: Option<crate::JsonValue>,
}

/// A single AP LAN port entry, returned in the list by
/// [`OmadaClient::ap_ports`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApLanPort {
    /// Port identifier.
    pub id: Option<String>,
    /// ONU identifier.
    pub onu_id: Option<String>,
    /// Port number.
    pub port: Option<i32>,
    /// Port type.
    pub port_type: Option<i32>,
    /// LAN port label (e.g. `"LAN1"`).
    pub lan_port: Option<String>,
    /// Port display name (1–64 ASCII characters). Defaults to `lan_port` when
    /// `None`.
    pub name: Option<String>,
    /// Link status. `1`: up; `0`: down.
    pub link_status: Option<i32>,
    /// Real-time link speed. `1`: 10 Mbps; `2`: 100 Mbps; `3`: 1000 Mbps;
    /// `4`: 10 Gbps.
    pub link_speed: Option<i32>,
    /// Real-time duplex mode. `1`: Half; `2`: Full.
    pub duplex: Option<i32>,
    /// Whether this port supports VLANs.
    pub support_vlan: Option<bool>,
    /// Whether local VLAN is enabled on this port.
    pub local_vlan_enable: Option<bool>,
    /// Local VLAN ID.
    pub local_vlan_id: Option<i32>,
}

/// Request body for [`OmadaClient::update_ap_port`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApLanPort {
    /// LAN port label (e.g. `"LAN1"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lan_port: Option<String>,
    /// Port display name (1–64 ASCII characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Port enabled state. `true`: enabled; `false`: disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<bool>,
    /// Whether local VLAN is enabled on this port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_vlan_enable: Option<bool>,
    /// Local VLAN ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_vlan_id: Option<i32>,
    /// Local VLAN network ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_vlan_network_id: Option<String>,
    /// Whether `PoE` output is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poe_out_enable: Option<bool>,
    /// Whether a custom VLAN configuration is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<bool>,
    /// Tagged VLAN IDs (comma-separated string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagged: Option<String>,
    /// Untagged VLAN IDs (comma-separated string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub untagged: Option<String>,
    /// Tagged network IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagged_network_id: Option<Vec<String>>,
}

/// VLAN summary entry returned by [`OmadaClient::ap_port_vlans_page`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApVlanSummary {
    /// VLAN ID.
    pub local_vlan_id: Option<i32>,
    /// LAN network ID.
    pub local_vlan_network_id: Option<String>,
    /// LAN network name.
    pub name: Option<String>,
    /// IP address for this VLAN.
    pub ipaddr: Option<String>,
    /// Ports in native (untagged access) mode.
    pub native_port: Option<Vec<String>>,
    /// Ports in tagged mode.
    pub tag_port: Option<Vec<String>>,
    /// Ports in untagged mode.
    pub untag_port: Option<Vec<String>>,
}

/// P2P bridge group information for an AP, returned by
/// [`OmadaClient::ap_p2p_info`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApP2pInfo {
    /// Main (root) AP of the bridge group.
    pub main_ap: Option<P2pDevice>,
    /// Child APs in the bridge group.
    pub child_aps: Option<Vec<P2pDevice>>,
}

/// A device entry within a P2P bridge group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct P2pDevice {
    /// Device MAC address.
    pub device_mac: Option<String>,
    /// Device name.
    pub device_name: Option<String>,
}

/// Mesh statistics for an AP, returned by [`OmadaClient::ap_mesh_statistics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMeshStatistics {
    /// AP connection status code.
    pub status: Option<i32>,
    /// AP connection status category.
    pub status_category: Option<i32>,
    /// Whether the AP has a wireless uplink.
    pub wireless_linked: Option<bool>,
    /// Mesh scan status. `0`: init; `1`: scanning; `2`: success; `3`: failed.
    pub scan_status: Option<i32>,
    /// Child APs connected to this AP.
    pub child_aps: Option<Vec<ChildAp>>,
    /// Wireless uplink information. Present when `wireless_linked` is `true`.
    pub wireless_uplink: Option<ApWirelessUplink>,
    /// Candidate parent APs for mesh association.
    pub candidate_parents: Option<Vec<CandidateParent>>,
}

/// A child AP connected via mesh to this AP, used in [`ApMeshStatistics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildAp {
    /// MAC address of the child AP.
    pub mac: Option<String>,
    /// Name of the child AP.
    pub name: Option<String>,
    /// RSSI of the mesh connection.
    pub rssi: Option<i32>,
    /// Device model.
    pub model: Option<String>,
    /// Device model version.
    pub model_version: Option<String>,
    /// Device IP address.
    pub ip: Option<String>,
    /// Device series type. `0`: advanced; `1`: pro.
    pub device_series_type: Option<i32>,
    /// Whether speed test is supported for this child AP.
    pub support_speed_test: Option<bool>,
    /// Device type.
    pub r#type: Option<String>,
}

/// A candidate parent AP for mesh association, used in [`ApMeshStatistics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateParent {
    /// Priority status of this parent candidate.
    pub priority_status: Option<i32>,
    /// MAC address of the candidate parent AP.
    pub mac: Option<String>,
    /// Name of the candidate parent AP.
    pub name: Option<String>,
    /// Link status. `0`: init; `1`: linking; `2`: linked; `3`: link failed;
    /// `4`: offline.
    pub link_status: Option<i32>,
    /// Number of mesh hops to this candidate.
    pub hop: Option<i32>,
    /// Number of child APs already connected to this candidate.
    pub childsnum: Option<i32>,
    /// Whether this AP supports 5G1 & 5G2 multi-band.
    pub support5g_multi_band: Option<bool>,
    /// Device model.
    pub model: Option<String>,
}

/// Bridge pairing window result for an AP, returned by
/// [`OmadaClient::ap_paring_window_result`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApBridgeParingWindowResult {
    /// Pairing window status.
    pub status: Option<i32>,
    /// Countdown timer in seconds.
    pub countdown: Option<i64>,
    /// Client APs discovered during the pairing window.
    pub client_aps: Option<Vec<ApBridgeClientAp>>,
}

/// A client AP discovered during bridge pairing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApBridgeClientAp {
    /// Client AP MAC address.
    pub mac: Option<String>,
    /// Connection status code.
    pub status: Option<i32>,
    /// RSSI of the connection.
    pub rssi: Option<i32>,
}

/// LLDP configuration for an AP, returned by [`OmadaClient::ap_lldp_config`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApLldpConfig {
    /// LLDP enable state. `0`: off; `1`: on; `2`: use site settings.
    pub lldp_enable: Option<i32>,
    /// Whether the LLDP feature is supported by this device.
    pub support_lldp: Option<bool>,
}

/// GPS-derived location coordinates for an AP, returned by
/// [`OmadaClient::ap_location_from_gps`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApLocationConfig {
    /// Longitude of the AP location.
    pub longitude: Option<f64>,
    /// Latitude of the AP location.
    pub latitude: Option<f64>,
}

// ── AP action / request types ─────────────────────────────────────────────────

/// Request body for [`OmadaClient::update_ap_wlan_group`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApWlanGroupRequest {
    /// ID of the WLAN group to switch the AP to. Must be in the same site and
    /// must not be the AP's current WLAN group.
    pub wlan_group_id: String,
}

/// Request body for [`OmadaClient::ap_start_speed_test`].
///
/// Initiates a P2P link speed test between two APs in a Main–Client
/// relationship.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApSpeedTestCommand {
    /// MAC address of either the Main AP or the Client AP. Both APs must be
    /// in a Main–Client relationship.
    pub child_mac: String,
}

/// Request body for [`OmadaClient::ap_full_channel_detect_start`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApFullChannelDetectRequest {
    /// Whether to enable Wi-Fi interference detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_wifi_interference: Option<bool>,
    /// Whether to enable channel load detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_channel_util: Option<bool>,
}

/// Request body for [`OmadaClient::aps_move_site`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApMoveSiteRequest {
    /// Target site ID to move the APs to.
    pub target_site: String,
    /// List of AP MAC addresses to move. When omitted, all APs in the source
    /// site are moved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ap_macs: Option<Vec<String>>,
}

/// Result of a batch site-move operation returned by
/// [`OmadaClient::aps_move_site`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveSiteResult {
    /// Batch move operation ID. Use the "Get batch move site" endpoint to
    /// poll this operation's result.
    pub move_site_id: Option<String>,
}

/// Request body for [`OmadaClient::ap_start_rf_scan`].
///
/// All fields are optional; omit the struct entirely (pass `&Default::default()`)
/// to scan all frequency bands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RfScanCommand {
    /// Frequency bands to scan. `0`: 2.4 GHz, `1`: 5 GHz, `2`: 5 GHz-2,
    /// `3`: 6 GHz. When absent or empty the AP scans all supported bands.
    /// Only honoured on devices that support per-band RF scan selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_id_list: Option<Vec<i32>>,
}

// ── LAN network ───────────────────────────────────────────────────────────────

/// A DHCP address range entry used in [`DhcpSettings`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpRange {
    /// DHCP range start IP.
    pub ipaddr_start: Option<String>,
    /// DHCP range end IP.
    pub ipaddr_end: Option<String>,
}

/// A custom DHCP option entry used in [`DhcpSettings`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpOption {
    /// Custom DHCP option code.
    pub code: Option<i32>,
    /// Type: `0`: String; `1`: IP Address; `2`: Hex Array.
    #[serde(rename = "type")]
    pub option_type: Option<i32>,
    /// Value.
    pub value: Option<String>,
}

/// Gateway DHCP server settings for a LAN network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpSettings {
    /// When `true`, the DHCP server is enabled.
    pub enable: Option<bool>,
    /// DHCP address ranges. Size must not exceed the `dhcp_range_pool_size`
    /// reported in [`LanNetworkPage`].
    pub ip_range_pool: Option<Vec<DhcpRange>>,
    /// Numeric representation of the gateway subnet start IP.
    pub ip_range_start: Option<i64>,
    /// Numeric representation of the gateway subnet end IP.
    pub ip_range_end: Option<i64>,
    /// DHCP name server mode: `"auto"` or `"manual"`.
    pub dhcpns: Option<String>,
    /// Primary DNS server (when `dhcpns` is `"manual"`).
    pub pri_dns: Option<String>,
    /// Secondary DNS server (when `dhcpns` is `"manual"`).
    pub snd_dns: Option<String>,
    /// Lease time in minutes (range: 2–10080).
    pub leasetime: Option<i32>,
    /// Manual DHCP gateway IP.
    pub gateway: Option<String>,
    /// Optional next DHCP server IP.
    pub dhcp_next_server: Option<String>,
    /// Custom DHCP options.
    pub options: Option<Vec<DhcpOption>>,
}

/// Legal DHCP server guard settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpGuard {
    /// Whether DHCP guarding is enabled.
    pub enable: bool,
    /// Fill mode: `1`: follow server; `2`: custom.
    pub mode: Option<i32>,
    /// DHCP server IP 1.
    pub dhcp_svr1: Option<String>,
    /// DHCP server IP 2.
    pub dhcp_svr2: Option<String>,
}

/// Legal `DHCPv6` server guard settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dhcpv6Guard {
    /// Whether `DHCPv6` guarding is enabled.
    pub enable: bool,
    /// `DHCPv6` server IPv6 address 1.
    pub dhcpv6_svr1: Option<String>,
    /// `DHCPv6` server IPv6 address 2.
    pub dhcpv6_svr2: Option<String>,
}

/// `DHCPv6` address pool settings (used in [`LanNetworkIpv6Config`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dhcpv6Settings {
    /// Gateway IPv6 address.
    pub gateway: Option<String>,
    /// IPv6 prefix length.
    pub subnet: Option<i32>,
    /// DHCP range start address.
    pub ipaddr_start: Option<String>,
    /// DHCP range end address.
    pub ipaddr_end: Option<String>,
    /// Lease time in minutes (range: 1–11520).
    pub leasetime: Option<i32>,
    /// DHCP name server mode: `0`: auto; `1`: manual.
    pub dnsv6: Option<i32>,
    /// Primary DNS server (when `dnsv6` is `1`).
    pub pri_dns: Option<String>,
    /// Secondary DNS server (when `dnsv6` is `1`).
    pub snd_dns: Option<String>,
}

/// SLAAC+Stateless DHCP or SLAAC+RDNSS mode settings (used in [`LanNetworkIpv6Config`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaacSettings {
    /// Prefix type: `0`: manual; `1`: get from Prefix Delegation.
    pub pre_type: Option<i32>,
    /// Address prefix.
    pub prefix: Option<String>,
    /// UUID of the WAN port.
    pub port_uuid: Option<String>,
    /// Prefix ID (range: 0–127).
    pub pre_id: Option<i32>,
    /// DHCP name server mode: `0`: auto; `1`: manual.
    pub dnsv6: Option<i32>,
    /// Primary DNS server (when `dnsv6` is `1`).
    pub pri_dns: Option<String>,
    /// Secondary DNS server (when `dnsv6` is `1`).
    pub snd_dns: Option<String>,
}

/// IPv6 Pass-Through mode settings (used in [`LanNetworkIpv6Config`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassThroughSettings {
    /// Port ID of the WAN port.
    pub port_id: Option<String>,
}

/// Router Advertisement settings (used in [`LanNetworkIpv6Config`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaSettings {
    /// Whether RA is enabled.
    pub enable: Option<bool>,
    /// Preference: `0`: low; `1`: medium; `2`: high.
    pub preference: Option<i32>,
    /// Valid lifetime. Must exceed `preferred_lifetime`.
    pub valid_lifetime: Option<i32>,
    /// Preferred lifetime.
    pub preferred_lifetime: Option<i32>,
}

/// LAN network IPv6 configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanNetworkIpv6Config {
    /// IPv6 connection type. `0`: none; `1`: `DHCPv6`; `2`: SLAAC+Stateless DHCP;
    /// `3`: SLAAC+RDNSS; `4`: passthrough.
    pub proto: i32,
    /// IPv6 enable: `0`: Disable; `1`: Enable.
    pub enable: i32,
    /// `DHCPv6` address pool settings (when `proto` is `1`).
    pub dhcpv6: Option<Dhcpv6Settings>,
    /// SLAAC+Stateless DHCP settings (when `proto` is `2`).
    pub slaac: Option<SlaacSettings>,
    /// SLAAC+RDNSS settings (when `proto` is `3`).
    pub rdnss: Option<SlaacSettings>,
    /// Pass-through settings (when `proto` is `4`).
    pub passthrough: Option<PassThroughSettings>,
    /// Router Advertisement settings.
    pub ra: Option<RaSettings>,
}

/// IP setting for a switch LAN interface.
///
/// Only valid when `device_type` is `2` in [`LanNetwork`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchIpSetting {
    /// IP setting mode: `0`: Static; `1`: DHCP.
    pub mode: i32,
    /// Static IP address (when `mode` is `0`), e.g. `"192.168.0.1"`.
    pub ip: Option<String>,
    /// Subnet mask (when `mode` is `0`), e.g. `"255.255.255.0"`.
    pub netmask: Option<String>,
    /// DHCP option 12 hostname.
    pub option12: Option<String>,
}

/// A DHCP address range used in [`SwitchDhcpServer`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchDhcpRange {
    /// Start IP of the range.
    pub start_ip: String,
    /// End IP of the range.
    pub end_ip: String,
}

/// A custom DHCP option used in [`SwitchDhcpServer`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchDhcpOption {
    /// Custom DHCP option code.
    pub code: Option<i32>,
    /// Type: `0`: Hex Array; `1`: String; `2`: IP Address.
    #[serde(rename = "type")]
    pub option_type: Option<i32>,
    /// Value.
    pub value: Option<String>,
}

/// DHCP server settings for a switch LAN interface.
///
/// Only valid when `device_type` is `2` and `mode` is `1` in [`LanNetwork`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchDhcpServer {
    /// DHCP server IP, e.g. `"192.168.0.1"`.
    pub ip: String,
    /// Subnet mask. Must not be within the range 1–30.
    pub netmask: String,
    /// DHCP address ranges.
    pub ip_range_pool: Option<Vec<SwitchDhcpRange>>,
    /// Primary DNS server.
    pub pri_dns: Option<String>,
    /// Secondary DNS server.
    pub snd_dns: Option<String>,
    /// Lease time in minutes (range: 2–2880).
    pub leasetime: i32,
    /// Gateway IP.
    pub gateway: Option<String>,
    /// DHCP option 138 IP.
    pub option138: Option<String>,
    /// Custom DHCP options.
    pub options: Option<Vec<SwitchDhcpOption>>,
    /// VRF ID.
    pub vrf_id: Option<String>,
}

/// DHCP relay settings for a switch LAN interface.
///
/// Only valid when `device_type` is `2` and `mode` is `2` in [`LanNetwork`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchDhcpRelay {
    /// Relay agent IP address, e.g. `"192.168.0.1"`.
    pub addr: Option<String>,
    /// VRF ID.
    pub vrf_id: Option<String>,
    /// DHCP server IP addresses.
    pub server_addrs: Option<Vec<String>>,
}

/// A LAN network returned by [`OmadaClient::lan_networks_page`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanNetwork {
    /// LAN network ID.
    pub id: Option<String>,
    /// Site ID.
    pub site: Option<String>,
    /// LAN network name (1 to 128 characters).
    pub name: String,
    /// LAN network purpose: `0`: VLAN; `1`: interface.
    pub purpose: i32,
    /// Gateway LAN port IDs (from "Check WAN/LAN status").
    pub interface_ids: Option<Vec<String>>,
    /// VLAN type (when `purpose` is interface): `0`: Single; `1`: Multiple.
    pub vlan_type: Option<i32>,
    /// VLAN string for batch creation (when `vlan_type` is `1`). Format: `"200, 1-100"`.
    pub vlans: Option<String>,
    /// Effective device type: `0`: Gateway and Switch; `1`: Switch.
    pub application: Option<i32>,
    /// Whether the network is isolated.
    pub isolation: Option<bool>,
    /// VLAN ID (when `vlan_type` is `0`), range 1–4094.
    pub vlan: Option<i32>,
    /// Gateway subnet in `IP/Mask` format (when `purpose` is interface).
    pub gateway_subnet: Option<String>,
    /// Gateway DHCP server settings.
    pub dhcp_settings: Option<DhcpSettings>,
    /// Domain of this network.
    pub domain: Option<String>,
    /// Whether IGMP snooping is enabled.
    pub igmp_snoop_enable: bool,
    /// IGMP snooping fast leave enable status.
    pub fast_leave_enable: Option<bool>,
    /// Whether MLD snooping is enabled.
    pub mld_snoop_enable: Option<bool>,
    /// Legal `DHCPv6` server guard settings.
    pub dhcpv6_guard: Option<Dhcpv6Guard>,
    /// Whether DHCP L2 relay is enabled.
    pub dhcp_l2_relay_enable: Option<bool>,
    /// Legal DHCP server guard settings.
    pub dhcp_guard: Option<DhcpGuard>,
    /// Whether a portal is configured on this network.
    pub portal: Option<bool>,
    /// Portal ID.
    pub portal_id: Option<String>,
    /// Portal name.
    pub portal_name: Option<String>,
    /// Whether an access control rule is active on this network.
    pub access_control_rule: Option<bool>,
    /// Whether a rate limit is active on this network.
    pub rate_limit: Option<bool>,
    /// LAN network IPv6 configuration.
    ///
    /// Note: the JSON field name contains a typo (`"lanNeworkIpv6Config"`); this
    /// is preserved verbatim so that the field round-trips correctly.
    #[serde(rename = "lanNeworkIpv6Config")]
    pub lan_network_ipv6_config: Option<LanNetworkIpv6Config>,
    /// When Internet pre-config is closed or Universal, `true`; after adopting gateway, `false`.
    pub all_lan: Option<bool>,
    /// Resource origin: `0`: new created; `1`: from template; `2`: override.
    pub resource: Option<i32>,
    /// Original name (from template).
    pub orig_name: Option<String>,
    /// Whether ARP detection is enabled. Only valid when `device_type` is `1`.
    pub arp_detection_enable: Option<bool>,
    /// Whether `QoS` queue is enabled.
    pub qos_queue_enable: Option<bool>,
    /// `QoS` queue ID.
    pub queue_id: Option<i32>,
    /// Whether VLAN type is Multiple.
    pub exist_multi_vlan: Option<bool>,
    /// Whether RA has been configured.
    pub exist_ra: Option<bool>,
    /// Whether custom DHCP options have been configured.
    pub exist_custom_dhcp_option: Option<bool>,
    /// Whether DHCP Next Server has been configured.
    pub exist_dhcp_next_server: Option<bool>,
    /// Whether ARP detection is configured.
    pub exist_arp_detection: Option<bool>,
    /// Whether network isolation is configured.
    pub exist_network_isolation: Option<bool>,
    /// DHCP server device type: `0`: External Device; `1`: gateway; `2`: switch; `3`: none.
    pub device_type: i32,
    /// DHCP server device MAC. Only valid when `device_type` is `1` or `2`.
    pub device_mac: Option<String>,
    /// DHCP server device stack ID. Only valid when `device_type` is `2` and the device is a stack.
    pub stack_id: Option<String>,
    /// Switch IP setting. Only valid when `device_type` is `2`.
    pub ip: Option<SwitchIpSetting>,
    /// DHCP mode. `0`: None; `1`: DHCP Server; `2`: DHCP Relay.
    /// Only valid when `device_type` is `2`.
    pub mode: Option<i32>,
    /// VRF ID.
    pub vrf_id: Option<String>,
    /// Switch DHCP server settings. Only valid when `device_type` is `2` and `mode` is `1`.
    pub dhcp_server: Option<SwitchDhcpServer>,
    /// Switch DHCP relay settings. Only valid when `device_type` is `2` and `mode` is `2`.
    pub dhcp_relay: Option<SwitchDhcpRelay>,
    /// Network delivering state: `0`: not delivering; `1`: delivering; `2`: deliver done.
    pub state: Option<i32>,
    /// Total IP count.
    pub total_ip_num: Option<i64>,
    /// Number of DHCP server devices in effect. Only valid when `vlan_type` is `0`.
    pub dhcp_server_num: Option<i32>,
    /// Whether DHCP settings subnet override is enabled.
    pub subnet_override_enable: Option<bool>,
    /// Whether subnet override is active.
    pub subnet_override: Option<bool>,
    /// Whether this is the primary network.
    pub primary: Option<bool>,
}

/// A paginated list of LAN networks together with site-level capability metadata,
/// returned by [`OmadaClient::lan_networks_page`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanNetworkPage {
    /// Total rows of all items.
    pub total_rows: i64,
    /// Current page number.
    pub current_page: i32,
    /// Number of entries per page.
    pub current_size: i32,
    /// LAN networks on this page.
    pub data: Vec<LanNetwork>,
    /// Whether multi-VLAN configuration is supported by the site.
    pub support_multi_vlan: Option<bool>,
    /// Whether Router Advertisement configuration is supported by the site.
    #[serde(rename = "supportRA")]
    pub support_ra: Option<bool>,
    /// Whether custom DHCP option configuration is supported by the site.
    pub support_custom_dhcp_option: Option<bool>,
    /// Whether the current site uses a combined gateway.
    pub combined_gateway: Option<bool>,
    /// Maximum number of DHCP ranges per network.
    pub dhcp_range_pool_size: Option<i32>,
    /// Number of gateway interfaces available in the site.
    pub interface_num: Option<i32>,
    /// Whether the Default VLAN may be selected.
    pub support_default: Option<bool>,
    /// Model names that support IGMP snooping in this site.
    pub osg_support_igmp_snooping: Option<Vec<String>>,
    /// Whether DHCP Next Server is supported by the site.
    pub support_dhcp_next_server: Option<bool>,
    /// Total number of VLANs in the site.
    pub vlan_nums: Option<i32>,
    /// Whether ARP detection is supported by the site.
    pub support_arp_detection: Option<bool>,
    /// Whether network isolation is supported by the site.
    pub support_network_isolation: Option<bool>,
    /// Whether LAN IPv6 is supported by the site.
    pub support_lan_ipv6: Option<bool>,
    /// Whether IPv6 Pass Through is supported by the site.
    pub support_lan_ipv6_pass_through: Option<bool>,
    /// Maximum number of VLANs supported by the gateway.
    pub support_max_vlan_num: Option<i32>,
    /// Whether "Get from Prefix Delegation" is available when Prefix Delegation
    /// is disabled on the corresponding WAN interface.
    pub support_pd_on_dhcp: Option<bool>,
}

// ── Device ────────────────────────────────────────────────────────────────────

/// A device returned by [`OmadaClient::devices_page`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// Device MAC.
    pub mac: Option<String>,
    /// Device name.
    pub name: Option<String>,
    /// Device type.
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    /// Switch subtype: `"smart"` (Non-Agile Series Switch) or `"es"` (Agile Series Switch).
    pub subtype: Option<String>,
    /// Device series type. `0`: basic; `1`: pro.
    pub device_series_type: Option<i32>,
    /// Device model name with version.
    pub model: Option<String>,
    /// Device model name.
    pub model_name: Option<String>,
    /// Device IP.
    pub ip: Option<String>,
    /// Device IPv6 list.
    pub ipv6: Option<Vec<String>>,
    /// Device uptime.
    pub uptime: Option<String>,
    /// Device status. `0`: Disconnected; `1`: Connected; `2`: Pending;
    /// `3`: Heartbeat Missed; `4`: Isolated.
    pub status: Option<i32>,
    /// Detailed device status. See API spec for the full set of values.
    pub detail_status: Option<i32>,
    /// Model version of the device, e.g. `"3.0"`.
    pub model_version: Option<String>,
    /// Last seen time (Unix timestamp, milliseconds).
    pub last_seen: Option<i64>,
    /// CPU utilisation percentage.
    pub cpu_util: Option<i32>,
    /// Memory utilisation percentage.
    pub mem_util: Option<i32>,
    /// Device serial number.
    pub sn: Option<String>,
    /// Device license status (cloud-based only). `0`: unActive; `1`: Unbind;
    /// `2`: Expired; `3`: active.
    pub license_status: Option<i32>,
    /// Device tag name.
    pub tag_name: Option<String>,
    /// Uplink device MAC address.
    pub uplink_device_mac: Option<String>,
    /// Uplink device name.
    pub uplink_device_name: Option<String>,
    /// Uplink device port.
    pub uplink_device_port: Option<String>,
    /// Device uplink port link speed. `0`: Auto; `1`: 10 M; `2`: 100 M;
    /// `3`: 1000 M; `4`: 2500 M; `5`: 10 G; `6`: 5 G; `7`: 25 G; `8`: 100 G.
    pub link_speed: Option<i32>,
    /// Device uplink port duplex mode. `0`: Auto; `1`: Half; `2`: Full.
    pub duplex: Option<i32>,
    /// Whether the device can be adopted by the site.
    pub switch_consistent: Option<bool>,
    /// Device public IP.
    pub public_ip: Option<String>,
    /// The device firmware version.
    pub firmware_version: Option<String>,
    /// The compatible type of device.
    pub compatible: Option<i32>,
    /// Indicates whether the device is activated.
    pub active: Option<bool>,
    /// Whether the device is in the white list.
    pub in_white_list: Option<bool>,
}

/// Optional query parameters for [`OmadaClient::devices_page`] and
/// [`OmadaClient::devices`].
#[derive(Debug, Clone, Default)]
pub struct DeviceListParams {
    /// Fuzzy search key; supports name, MAC, and IP.
    pub search_key: Option<String>,
    /// Sort by name: `"asc"` or `"desc"`.
    pub sort_name: Option<String>,
    /// Sort by status: `"asc"` or `"desc"`.
    pub sort_status: Option<String>,
    /// Sort by IP: `"asc"` or `"desc"`.
    pub sort_ip: Option<String>,
    /// Filter by tag name.
    pub filter_tag: Option<String>,
}

// ── LAN profile ───────────────────────────────────────────────────────────────

/// Spanning-tree port settings used in [`LanProfileConfig`] and [`LanProfile`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanningTreeSetting {
    /// Port priority (range: 0–240).
    pub priority: i32,
    /// External path cost (range: 0–2000000).
    pub ext_path_cost: i32,
    /// Internal path cost (range: 0–2000000).
    pub int_path_cost: i32,
    /// Whether the port is an edge port.
    pub edge_port: bool,
    /// P2P link mode (range: 0–2).
    pub p2p_link: i32,
    /// Whether `MCheck` is enabled.
    pub mcheck: Option<bool>,
    /// Whether loop protect is enabled.
    pub loop_protect: Option<bool>,
    /// Whether root protect is enabled.
    pub root_protect: Option<bool>,
    /// Whether TC guard is enabled.
    pub tc_guard: Option<bool>,
    /// Whether BPDU protect is enabled.
    pub bpdu_protect: Option<bool>,
    /// Whether BPDU filter is enabled.
    pub bpdu_filter: Option<bool>,
    /// Whether BPDU forward is enabled.
    pub bpdu_forward: Option<bool>,
    /// Whether per-instance settings are enabled.
    pub instance_enable: Option<bool>,
    /// Per-instance priority configuration.
    pub instances: Option<Vec<SpanningTreeInstance>>,
}

/// A spanning-tree instance entry (MSTP or RPVST).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanningTreeInstance {
    /// STP type: `0`: MSTP; `1`: RPVST.
    pub stp: Option<i32>,
    /// MSTP instance ID.
    pub id: Option<i32>,
    /// RPVST VLAN ID string.
    pub vlan: Option<String>,
    /// Instance priority.
    pub priority: Option<i32>,
}

/// Bandwidth rate-limit control settings used in [`LanProfileConfig`] and [`LanProfile`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandCtrl {
    /// Whether egress rate limiting is enabled.
    pub egress_enable: bool,
    /// Egress rate limit value.
    pub egress_limit: Option<i32>,
    /// Egress rate unit: `1`: Kbps; `2`: Mbps.
    pub egress_unit: Option<i32>,
    /// Whether ingress rate limiting is enabled.
    pub ingress_enable: bool,
    /// Ingress rate limit value.
    pub ingress_limit: Option<i32>,
    /// Ingress rate unit: `1`: Kbps; `2`: Mbps.
    pub ingress_unit: Option<i32>,
}

/// Storm-control settings used in [`LanProfileConfig`] and [`LanProfile`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StormCtrl {
    /// Rate mode: `0`: ratio; `1`: kbps.
    pub rate_mode: Option<i32>,
    /// Whether unknown-unicast storm control is enabled.
    pub unknown_unicast_enable: bool,
    /// Unknown-unicast threshold.
    pub unknown_unicast: Option<i32>,
    /// Whether multicast storm control is enabled.
    pub multicast_enable: bool,
    /// Multicast threshold.
    pub multicast: Option<i32>,
    /// Whether broadcast storm control is enabled.
    pub broadcast_enable: bool,
    /// Broadcast threshold.
    pub broadcast: Option<i32>,
    /// Action on storm: `0`: drop (default); `1`: shutdown.
    pub action: i32,
    /// Recover time in seconds (range: 1–3600; default: 3600).
    pub recover_time: Option<i32>,
}

/// DHCP L2 relay settings used in [`LanProfileConfig`] and [`LanProfile`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpL2Relay {
    /// Whether DHCP L2 relay is enabled.
    pub enable: Option<bool>,
    /// Format: `0`: normal; `1`: private.
    pub format: Option<i32>,
}

/// Request/response body for LAN profile create and modify operations.
///
/// Used by [`OmadaClient::create_lan_profile`] and
/// [`OmadaClient::update_lan_profile`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct LanProfileConfig {
    /// Profile name (1 to 128 characters).
    pub name: String,
    /// `PoE` setting: `0`: on; `1`: off; `2`: do not modify.
    pub poe: i32,
    /// Native network ID. Cannot appear in tagged or untagged network lists.
    pub native_network_id: String,
    /// Tagged network IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_network_ids: Option<Vec<String>>,
    /// Untagged network IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub untag_network_ids: Option<Vec<String>>,
    /// Voice network ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_network_id: Option<String>,
    /// 802.1X authentication mode: `0`: force unauthorized; `1`: force authorized; `2`: auto.
    pub dot1x: i32,
    /// Whether port isolation is enabled.
    pub port_isolation_enable: bool,
    /// Whether LLDP-MED is enabled.
    pub lldp_med_enable: bool,
    /// Bandwidth control type: `0`: off; `1`: rate limit; `2`: storm control.
    pub band_width_ctrl_type: i32,
    /// Storm control settings (when `band_width_ctrl_type` is `2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storm_ctrl: Option<StormCtrl>,
    /// Rate limit settings (when `band_width_ctrl_type` is `1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub band_ctrl: Option<BandCtrl>,
    /// Whether spanning tree is enabled.
    pub spanning_tree_enable: bool,
    /// Spanning-tree port settings (when `spanning_tree_enable` is `true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spanning_tree_setting: Option<SpanningTreeSetting>,
    /// Whether loopback detection is enabled.
    pub loopback_detect_enable: bool,
    /// Whether EEE (Energy-Efficient Ethernet) is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eee_enable: Option<bool>,
    /// Whether flow control is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_control_enable: Option<bool>,
    /// Whether VLAN-based loopback detection is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loopback_detect_vlan_based_enable: Option<bool>,
    /// Whether IGMP fast leave is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub igmp_fast_leave_enable: Option<bool>,
    /// Whether MLD fast leave is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mld_fast_leave_enable: Option<bool>,
    /// DHCP L2 relay settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhcp_l2_relay_settings: Option<DhcpL2Relay>,
    /// IGMP snooping fast leave enable status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_leave_enable: Option<bool>,
    /// 802.1p priority.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dot1p_priority: Option<i32>,
    /// Trust mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_mode: Option<i32>,
}

/// A LAN profile returned by [`OmadaClient::lan_profiles_page`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct LanProfile {
    /// LAN profile ID.
    pub id: Option<String>,
    /// Profile flag: `0`: default (all/disable/LAN); `1`: native (generated when
    /// creating a LAN network); `2`: custom (created by users).
    pub flag: Option<i32>,
    /// Profile name (1 to 128 characters).
    pub name: String,
    /// `PoE` setting: `0`: on; `1`: off; `2`: do not modify.
    pub poe: i32,
    /// Native network ID.
    pub native_network_id: Option<String>,
    /// Tagged network IDs.
    pub tag_network_ids: Option<Vec<String>>,
    /// Untagged network IDs.
    pub untag_network_ids: Option<Vec<String>>,
    /// Voice network ID.
    pub voice_network_id: Option<String>,
    /// 802.1X authentication mode: `0`: force unauthorized; `1`: force authorized; `2`: auto.
    pub dot1x: i32,
    /// Whether port isolation is enabled.
    pub port_isolation_enable: bool,
    /// Whether LLDP-MED is enabled.
    pub lldp_med_enable: bool,
    /// Whether topology-change notify is enabled.
    pub topo_notify_enable: Option<bool>,
    /// Bandwidth control type: `0`: off; `1`: rate limit; `2`: storm control.
    pub band_width_ctrl_type: i32,
    /// Storm control settings.
    pub storm_ctrl: Option<StormCtrl>,
    /// Rate limit settings.
    pub band_ctrl: Option<BandCtrl>,
    /// Whether spanning tree is enabled.
    pub spanning_tree_enable: bool,
    /// Spanning-tree port settings.
    pub spanning_tree_setting: Option<SpanningTreeSetting>,
    /// Whether loopback detection is enabled.
    pub loopback_detect_enable: bool,
    /// Whether EEE (Energy-Efficient Ethernet) is enabled.
    pub eee_enable: Option<bool>,
    /// Whether flow control is enabled.
    pub flow_control_enable: Option<bool>,
    /// Whether VLAN-based loopback detection is enabled.
    pub loopback_detect_vlan_based_enable: Option<bool>,
    /// Whether IGMP fast leave is enabled.
    pub igmp_fast_leave_enable: Option<bool>,
    /// Whether MLD fast leave is enabled.
    pub mld_fast_leave_enable: Option<bool>,
    /// DHCP L2 relay settings.
    pub dhcp_l2_relay_settings: Option<DhcpL2Relay>,
    /// IGMP snooping fast leave enable status.
    pub fast_leave_enable: Option<bool>,
    /// 802.1p priority.
    pub dot1p_priority: Option<i32>,
    /// Trust mode.
    pub trust_mode: Option<i32>,
    /// Profile type: `0`: LAN profile-ALL; `1`: LAN profile-Disable;
    /// `2`: LAN profile (other).
    #[serde(rename = "type")]
    pub profile_type: Option<i32>,
    /// Whether Agile Series Switch support is enabled.
    pub support_es_enable: Option<bool>,
    /// Whether the VLAN config in this profile conflicts with the effective VLAN
    /// config of the bound port.
    pub network_conflict: Option<bool>,
}

// ── Switch networks ───────────────────────────────────────────────────────────

/// IP address range entry for a DHCP server pool.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OswDhcpServerRange {
    /// Start of the DHCP address range (e.g. `"192.168.1.100"`).
    pub start_ip: String,
    /// End of the DHCP address range (e.g. `"192.168.1.200"`).
    pub end_ip: String,
}

/// Custom DHCP option entry for a switch DHCP server.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OswDhcpCustomOption {
    /// Custom DHCP option code.
    pub code: Option<i32>,
    /// Type: `0` = Hex Array; `1` = String; `2` = IP Address.
    #[serde(rename = "type")]
    pub option_type: Option<i32>,
    /// Option value.
    pub value: Option<String>,
}

/// DHCP server configuration for a switch VLAN interface.
///
/// Only valid when `mode` is `1` on the parent [`SwitchNetwork`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OswDhcpServer {
    /// DHCP server IP (e.g. `"192.168.0.1"`).
    pub ip: String,
    /// Subnet mask; prefix length must be within 1–30.
    pub netmask: String,
    /// Address pool ranges.
    pub ip_range_pool: Option<Vec<OswDhcpServerRange>>,
    /// Primary DNS server.
    pub pri_dns: Option<String>,
    /// Secondary DNS server.
    pub snd_dns: Option<String>,
    /// Lease time in minutes; must be within 2–2880.
    pub leasetime: i32,
    /// Default gateway IP.
    pub gateway: Option<String>,
    /// DHCP option 138 (CAPWAP controller address).
    pub option138: Option<String>,
    /// Custom DHCP options.
    pub options: Option<Vec<OswDhcpCustomOption>>,
    /// VRF ID.
    pub vrf_id: Option<String>,
}

/// DHCP relay configuration for a switch VLAN interface.
///
/// Only valid when `mode` is `2` on the parent [`SwitchNetwork`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OswDhcpRelay {
    /// Relay server address (e.g. `"192.168.0.1"`).
    pub addr: Option<String>,
    /// VRF ID.
    pub vrf_id: Option<String>,
    /// Additional relay server addresses.
    pub server_addrs: Option<Vec<String>>,
}

/// IPv4 interface settings for a switch VLAN interface.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OswIpSetting {
    /// IP setting mode: `0` = Static; `1` = DHCP.
    pub mode: i32,
    /// Static IP address (mode `0`), e.g. `"192.168.0.1"`.
    pub ip: Option<String>,
    /// Default gateway (e.g. `"192.168.137.1"`).
    pub gateway: Option<String>,
    /// Subnet mask (e.g. `"255.255.255.0"`).
    pub netmask: Option<String>,
    /// Primary DNS server.
    pub pre_dns: Option<String>,
    /// Secondary DNS server.
    pub sec_dns: Option<String>,
    /// Whether to use a fallback IP in DHCP mode.
    pub fallback: Option<bool>,
    /// Fallback static IP for DHCP mode when `fallback` is enabled.
    pub fallback_ip: Option<String>,
    /// Fallback subnet mask for DHCP mode when `fallback` is enabled.
    pub fallback_mask: Option<String>,
    /// Fallback gateway for DHCP mode when `fallback` is enabled.
    pub fallback_gate: Option<String>,
    /// DHCP option 12 (hostname).
    pub option12: Option<String>,
    /// Whether to use a fixed (reserved) DHCP address.
    pub use_fixed_addr: Option<bool>,
    /// LAN network ID for address reservation when `use_fixed_addr` is enabled.
    pub net_id: Option<String>,
    /// Reserved DHCP IP address when `use_fixed_addr` is enabled.
    pub dhcp_ip: Option<String>,
    /// DHCP server type for address reservation.
    pub server_type: Option<String>,
    /// DHCP server MAC for address reservation.
    pub server_mac: Option<String>,
    /// DHCP server stack ID for address reservation.
    pub server_stack_id: Option<String>,
    /// Whether to enable IP-MAC conflict detection.
    pub confirm_conflict: Option<bool>,
}

/// IPv6 interface settings for a switch VLAN interface.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OswIpv6Setting {
    /// IPv6 setting mode: `"dynamic"` or `"static"`.
    pub mode: String,
    /// DNS mode for dynamic mode: `0` = Get Dynamic DNS; `1` = Use the following DNS.
    pub dns_mode: Option<i32>,
    /// Primary DNS server (e.g. `"2001:4860:4860::8888"`).
    pub pri_dns: Option<String>,
    /// Secondary DNS server (e.g. `"2001:4860:4860::8844"`).
    pub snd_dns: Option<String>,
    /// IPv6 address for static mode.
    pub ipv6_addr: Option<String>,
    /// Prefix length for static mode; must be within 1–128.
    pub prefix_len: Option<i32>,
}

/// VLAN interface configuration for a switch network.
///
/// Used both as a response item from [`OmadaClient::switch_networks_page`]
/// and as the request body for [`OmadaClient::update_switch_network`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchNetwork {
    /// Network ID.
    pub id: String,
    /// VLAN ID.
    pub vlan: i32,
    /// Whether the VLAN interface is enabled.
    pub status: Option<bool>,
    /// Whether this VLAN is the management VLAN.
    pub mvlan: bool,
    /// Switch network name.
    pub name: Option<String>,
    /// IPv4 interface settings.
    pub ip: Option<OswIpSetting>,
    /// Whether IPv6 is enabled on this interface.
    pub ipv6_enable: Option<bool>,
    /// IPv6 interface settings.
    pub ipv6: Option<OswIpv6Setting>,
    /// DHCP mode: `0` = None; `1` = DHCP Server; `2` = DHCP Relay.
    pub mode: i32,
    /// DHCP server settings. Only valid when `mode` is `1`.
    pub dhcp_server: Option<OswDhcpServer>,
    /// DHCP relay settings. Only valid when `mode` is `2`.
    pub dhcp_relay: Option<OswDhcpRelay>,
    /// VRF ID.
    pub vrf_id: Option<String>,
}

/// Paginated list of switch VLAN interfaces returned by
/// [`OmadaClient::switch_networks_page`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchNetworksPage {
    /// Total rows of all items.
    pub total_rows: i64,
    /// Current page number.
    pub current_page: i32,
    /// Number of entries per page.
    pub current_size: i32,
    /// VLAN interface items on this page.
    pub data: Vec<SwitchNetwork>,
    /// Whether the switch supports IPv6.
    pub support_ipv6: Option<bool>,
}

// ── Switch port config ────────────────────────────────────────────────────────

/// Stack unit/slot/port address for a switch port in a stacked topology.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchStandPort {
    /// Stack unit number.
    pub unit: i32,
    /// Slot number.
    pub slot: i32,
    /// Port number.
    pub port: i32,
}

/// M-LAG peer device settings within a [`SwitchLagSetting`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchMlagPeerSetting {
    /// Peer device MAC address.
    pub mac: Option<String>,
    /// Peer device LAG port numbers.
    pub mlag_peer_ports: Option<Vec<i32>>,
    /// Peer device LAG standard ports (stack topologies).
    pub mlag_peer_standard_ports: Option<Vec<SwitchStandPort>>,
}

/// LAG (Link Aggregation Group) configuration for a switch port.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchLagSetting {
    /// LAG ID.
    pub lag_id: i32,
    /// Member port numbers.
    pub ports: Option<Vec<i32>>,
    /// LAG type: `1`: Static LAG; `2`: LACP; `3`: LACP-Active; `4`: LACP-Passive.
    pub lag_type: Option<i32>,
    /// M-LAG group name.
    pub mlag_name: Option<String>,
    /// Whether the M-LAG port is enabled.
    pub mlag_enable: Option<bool>,
    /// M-LAG peer device settings.
    pub mlag_peer_setting: Option<SwitchMlagPeerSetting>,
}

/// IP-MAC binding entry for a switch port.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchPortImpb {
    /// Binding entry ID.
    pub id: Option<String>,
    /// Bound IP address.
    pub ip: Option<String>,
    /// Bound MAC address.
    pub mac: Option<String>,
    /// Client name.
    pub client_name: Option<String>,
    /// VLAN ID.
    pub vlan: Option<i32>,
}

/// DHCP L2 relay settings for a switch port.
///
/// Extends the basic relay settings with circuit-ID and remote-ID option
/// support.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchPortDhcpL2Relay {
    /// Whether DHCP L2 relay is enabled.
    pub enable: Option<bool>,
    /// Option 82 format: `0`: Normal; `1`: Private.
    pub format: Option<i32>,
    /// Circuit ID string (up to 64 characters, alphanumeric and `-_@:/.#`).
    pub circuit_id: Option<String>,
    /// Remote ID string (up to 64 characters, alphanumeric and `-_@:/.#`).
    pub remote_id: Option<String>,
}

/// Request body for [`OmadaClient::update_switch_port`] (PATCH `.../ports/{port}`).
///
/// All fields are optional — send only the fields you want to change.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct SwitchPortConfig {
    /// Port or LAG name.
    pub name: Option<String>,
    /// Tag ID list.
    pub tag_ids: Option<Vec<String>>,
    /// Native (untagged ingress) network ID.
    pub native_network_id: Option<String>,
    /// Native network bridge VLAN.
    pub native_bridge_vlan: Option<i32>,
    /// Tagged network setting: `0`: Allow All; `1`: Block All; `2`: Custom.
    pub network_tags_setting: Option<i32>,
    /// Tagged network IDs (when `network_tags_setting` is `2`).
    pub tag_network_ids: Option<Vec<String>>,
    /// Untagged (egress) network IDs.
    pub untag_network_ids: Option<Vec<String>>,
    /// Whether voice network is enabled.
    pub voice_network_enable: Option<bool>,
    /// Voice network ID.
    pub voice_network_id: Option<String>,
    /// Voice network bridge VLAN.
    pub voice_bridge_vlan: Option<i32>,
    /// Whether voice DSCP is enabled.
    pub voice_dscp_enable: Option<bool>,
    /// Voice DSCP value.
    pub voice_dscp: Option<i32>,
    /// Whether port alert is enabled.
    pub port_alert_enable: Option<bool>,
    /// FEC mode: `1`: Off; `2`: RS528; `3`: RS544; `4`: Auto; `5`: Base-R.
    pub fec_mode: Option<i32>,
    /// Whether the FEC mode is synchronously applied to the link peer port.
    pub fec_link_peer_apply_enable: Option<bool>,
    /// Whether the port is disabled.
    pub disable: Option<bool>,
    /// Profile ID to bind.
    pub profile_id: Option<String>,
    /// Whether profile override (custom fill mode) is enabled.
    pub profile_override_enable: Option<bool>,
    /// Whether VLAN configuration override is enabled.
    pub profile_vlan_override_enable: Option<bool>,
    /// Link speed: `0`: auto; `1`: 10M; `2`: 100M; `3`: 1000M; `4`: 2.5G; `5`: 10G.
    pub link_speed: Option<i32>,
    /// Duplex: `0`: Auto; `1`: Half; `2`: Full.
    pub duplex: Option<i32>,
    /// Whether IGMP snooping is enabled.
    pub igmp_snooping_enable: Option<bool>,
    /// Bandwidth control type: `0`: Off; `1`: Rate Limit; `2`: Storm Control.
    pub band_width_ctrl_type: Option<i32>,
    /// Rate-limit settings (used when `band_width_ctrl_type` is `1`).
    pub band_ctrl: Option<BandCtrl>,
    /// Storm-control settings (used when `band_width_ctrl_type` is `2`).
    pub storm_ctrl: Option<StormCtrl>,
    /// Whether spanning tree is enabled.
    pub spanning_tree_enable: Option<bool>,
    /// Spanning-tree port settings.
    pub spanning_tree_setting: Option<SpanningTreeSetting>,
    /// Whether port-based loopback detection is enabled.
    pub loopback_detect_enable: Option<bool>,
    /// Whether VLAN-based loopback detection is enabled.
    pub loopback_detect_vlan_based_enable: Option<bool>,
    /// Whether IGMP fast-leave is enabled.
    pub igmp_fast_leave_enable: Option<bool>,
    /// Whether MLD fast-leave is enabled.
    pub mld_fast_leave_enable: Option<bool>,
    /// Whether DHCP snooping is enabled.
    pub dhcp_snoop_enable: Option<bool>,
    /// Whether ARP detection is enabled.
    pub arp_detect_enable: Option<bool>,
    /// IP-MAC binding entries.
    pub impbs: Option<Vec<SwitchPortImpb>>,
    /// Whether port isolation is enabled.
    pub port_isolation_enable: Option<bool>,
    /// Whether Energy Efficient Ethernet is enabled.
    pub eee_enable: Option<bool>,
    /// Whether flow control is enabled.
    pub flow_control_enable: Option<bool>,
    /// Whether IGMP snooping fast-leave is enabled.
    pub fast_leave_enable: Option<bool>,
    /// DHCP L2 relay settings.
    pub dhcp_l2_relay_settings: Option<SwitchPortDhcpL2Relay>,
    /// 802.1p priority.
    pub dot1p_priority: Option<i32>,
    /// `QoS` trust mode: `0`: Untrusted; `1`: Trust 802.1p; `2`: Trust DSCP.
    pub trust_mode: Option<i32>,
    /// Whether `QoS` scheduling queue configuration is enabled (Agile Series).
    pub qos_queue_enable: Option<bool>,
    /// `QoS` scheduling queue ID (Agile Series).
    pub queue_id: Option<i32>,
    /// Port operation mode: `"switching"`, `"mirroring"`, or `"aggregating"`.
    pub operation: Option<String>,
    /// Mirrored (monitored) port numbers.
    pub mirrored_ports: Option<Vec<i32>>,
    /// Mirrored (monitored) LAG IDs.
    pub mirrored_lags: Option<Vec<i32>>,
    /// LAG configuration (used when `operation` is `"aggregating"`).
    pub lag_setting: Option<SwitchLagSetting>,
    /// 802.1X authentication mode: `0`: Force unauthorized; `1`: Force authorized; `2`: Auto.
    pub dot1x: Option<i32>,
    /// `PoE` mode: `0`: Off; `1`: 802.3at/af.
    pub poe: Option<i32>,
    /// Whether LLDP-MED is enabled.
    pub lldp_med_enable: Option<bool>,
    /// Whether topology change notification is enabled.
    pub topo_notify_enable: Option<bool>,
}

// ── Switch general config ──────────────────────────────────────────────────

/// An SDM (Software-Defined Memory) application entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdmApplication {
    /// Feature category: `0`: ACL & `QoS` (IPv4); `1`: ACL & `QoS` (IPv6);
    /// `3`: IP Source Guard; `4`: `IPv6` Source Guard.
    pub category: Option<i32>,
    /// Maximum number of entries allowed by the feature.
    pub num: Option<i32>,
}

/// A named SDM template with its per-feature entry limits.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdmTemplate {
    /// SDM template name.
    pub name: Option<String>,
    /// Applications supported by this template.
    pub apps: Option<Vec<SdmApplication>>,
}

/// SDM (Software-Defined Memory) template configuration for a switch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchSdm {
    /// SDM template currently in use.
    pub in_use: Option<String>,
    /// All SDM templates supported by the device.
    pub templates: Option<Vec<SdmTemplate>>,
}

// ── Switch info ───────────────────────────────────────────────────────────────

/// Port-level information for a switch port.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchPortInfo {
    /// Port ID.
    pub port: i32,
    /// Port name.
    pub name: String,
    /// Profile ID bound to this port.
    pub profile_id: String,
    /// Profile name bound to this port.
    pub profile_name: String,
    /// Whether profile override is enabled on this port.
    pub profile_override_enable: bool,
    /// `PoE` mode: `1` = on (802.3at/af); `0` = off.
    pub poe_mode: i32,
    /// Whether this port is part of a LAG.
    pub lag_port: bool,
    /// Port link status: `0` = off; `1` = on (only valid when `lag_port` is `false`).
    pub status: i32,
}

/// Overview information for a managed switch, as returned by `switch()`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchInfo {
    /// Switch MAC address.
    pub mac: Option<String>,
    /// Switch IP address.
    pub ip: Option<String>,
    /// Switch IPv6 address list.
    pub ipv6_list: Option<Vec<String>>,
    /// Device model.
    pub model: Option<String>,
    /// Full firmware version string (e.g. `"2.5.0 Build 20190118 Rel. 64821"`).
    pub firmware_version: Option<String>,
    /// Short firmware version string (e.g. `"2.5.0"`).
    pub version: Option<String>,
    /// Hardware version.
    pub hw_version: Option<String>,
    /// CPU utilization percentage.
    pub cpu_util: Option<i32>,
    /// Memory utilization percentage.
    pub mem_util: Option<i32>,
    /// Human-readable uptime string.
    pub uptime: Option<String>,
    /// Per-port status and configuration.
    pub port_list: Option<Vec<SwitchPortInfo>>,
}

// ── Wi-Fi Calling profiles ────────────────────────────────────────────────────

/// An ePDG (Evolved Packet Data Gateway) entry within a [`WifiCallingCarrier`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiCallingEpdg {
    /// QoS priority.
    #[allow(clippy::doc_markdown)]
    pub qos_priority: Option<i32>,
    /// ePDG type: `0` = domain, `1` = IP address.
    #[serde(rename = "type")]
    pub epdg_type: Option<i32>,
    /// Domain name. Required when `epdg_type` is `0`.
    pub domain: Option<String>,
    /// IP address. Required when `epdg_type` is `1`.
    pub ip: Option<String>,
}

/// A carrier entry within a Wi-Fi Calling profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiCallingCarrier {
    /// Carrier name.
    pub carrier_name: Option<String>,
    /// List of ePDG (Evolved Packet Data Gateway) entries for this carrier.
    pub epdgs: Option<Vec<WifiCallingEpdg>>,
}

/// A Wi-Fi Calling profile as returned by [`OmadaClient::wifi_calling_profiles`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiCallingProfile {
    /// Wi-Fi Calling Profile ID.
    pub id: Option<String>,
    /// Wi-Fi Calling Profile Name (1 to 32 UTF-8 characters).
    pub name: Option<String>,
    /// Description of the Wi-Fi Calling Profile (1 to 32 UTF-8 characters).
    pub description: Option<String>,
    /// List of carrier entries.
    pub carrier_list: Option<Vec<WifiCallingCarrier>>,
    /// Profile creation resource: `0` = newly created, `1` = from template,
    /// `2` = override template.
    pub resource: Option<i32>,
}

/// Request body for [`OmadaClient::create_wifi_calling_profile`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWifiCallingProfileRequest {
    /// Wi-Fi Calling Profile Name (1 to 32 UTF-8 characters).
    pub name: String,
    /// Description of the Wi-Fi Calling profile (1 to 32 UTF-8 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// List of carrier entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier_list: Option<Vec<WifiCallingCarrier>>,
}

/// Request body for [`OmadaClient::copy_wifi_calling_profile`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyWifiCallingProfileRequest {
    /// Name for the new (copied) Wi-Fi Calling Profile.
    pub name: String,
}

/// General configuration for a managed switch.
///
/// After the device is added to a stack group, only the device name may be
/// modified.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchGeneralConfig {
    /// Device name; must contain 1 to 128 characters and must not start or
    /// end with certain special characters.
    pub name: Option<String>,
    /// LED setting: `0`: off; `1`: on; `2`: Use Site Settings.
    pub led_setting: Option<i32>,
    /// Tag IDs bound to the device.
    pub tag_ids: Option<Vec<String>>,
    /// Physical location of the device.
    pub location: Option<DeviceLocation>,
    /// Jumbo-frame MTU in bytes; must be within the range 1518–9216.
    pub jumbo: Option<i32>,
    /// LAG hash algorithm: `0`: SRC MAC; `1`: DST MAC; `2`: SRC MAC + DST MAC;
    /// `3`: SRC IP; `4`: DST IP; `5`: SRC IP + DST IP.
    pub lag_hash_alg: Option<i32>,
    /// SDM template configuration.
    pub sdm: Option<SwitchSdm>,
}

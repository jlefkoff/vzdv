use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::utils::GENERAL_HTTP_CLIENT;

const BASE_URL: &str = "https://api.vatusa.net/facility";

pub enum MembershipType {
    Home,
    Visit,
    Both,
}

#[derive(Debug, Deserialize)]
pub struct VatusaRosterData {
    pub data: Vec<RosterMember>,
}

#[derive(Debug, Deserialize)]
pub struct RosterMember {
    pub cid: u32,
    pub fname: String,
    pub lname: String,
    pub email: String,
    pub facility: String,
    pub rating: u8,
    pub created_at: String,
    pub updated_at: String,
    #[serde(rename = "flag_needbasic")]
    pub flag_need_basic: bool,
    #[serde(rename = "flag_xferOverride")]
    pub flag_transfer_override: bool,
    pub facility_join: String,
    #[serde(rename = "flag_homecontroller")]
    pub flag_home_controller: bool,
    #[serde(rename = "lastactivity")]
    pub last_activity: String,
    #[serde(rename = "flag_broadcastOptedIn")]
    pub flag_broadcast_opted_in: bool,
    #[serde(rename = "flag_preventStaffAssign")]
    pub flag_prevent_staff_assign: Option<serde_json::Value>, // ?
    pub last_cert_sync: String,
    #[serde(rename = "flag_nameprivacy")]
    pub flag_name_privacy: bool,
    pub promotion_eligible: bool,
    pub transfer_eligible: Option<serde_json::Value>, // ?
    pub roles: Vec<serde_json::Value>,                // ?
    pub rating_short: String,
    #[serde(rename = "isMentor")]
    pub is_mentor: bool,
    #[serde(rename = "isSupIns")]
    pub is_sup_ins: bool,
    pub last_promotion: String,
    pub membership: String,
}

/// Get the roster of a VATUSA facility.
pub async fn get_roster(facility: &str, membership: MembershipType) -> Result<VatusaRosterData> {
    let mem_str = match membership {
        MembershipType::Home => "home",
        MembershipType::Visit => "visit",
        MembershipType::Both => "both",
    };
    let resp = GENERAL_HTTP_CLIENT
        .get(format!("{BASE_URL}/facility/{facility}/roster/{mem_str}"))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Got status {} from VATUSA roster API",
            resp.status().as_u16()
        ));
    }
    let data = resp.json().await?;
    Ok(data)
}

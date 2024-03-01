use crate::shared::AppState;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

// const VATSIM_OAUTH_URL_BASE: &str = "https://auth.vatsim.net/";
const VATSIM_OAUTH_URL_BASE: &str = "https://auth-dev.vatsim.net/";

#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub scopes: Vec<String>,
    pub token_type: String,
    pub expires_in: u64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponse {
    pub data: UserInfoResponseData,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponseData {
    pub cid: String,
    pub personal: UserInfoResponseDataPersonal,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponseDataPersonal {
    pub name_first: String,
    pub name_last: String,
    pub name_full: String,
    pub email: String,
}

/// Exchange the code from VATSIM OAuth for an access token.
pub async fn code_to_user_info(code: &str, state: &Arc<AppState>) -> Result<TokenResponse> {
    let client = reqwest::ClientBuilder::new().build()?;
    let resp = client
        .post(format!("{VATSIM_OAUTH_URL_BASE}oauth/token"))
        .json(&json!({
            "grant_type": "authorization_code",
            "client_id": state.config.vatsim.oauth_client_id,
            "client_secret": state.config.vatsim.oauth_client_secret,
            "redirect_uri": state.config.vatsim.oauth_client_calback_url,
            "code": code
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Got status code {} from VATSIM OAuth exchange",
            resp.status().as_u16()
        ));
    }
    let data = resp.json().await?;
    Ok(data)
}

/// Using the user's access token, get their VATSIM info.
pub async fn get_user_info(access_token: &str) -> Result<UserInfoResponse> {
    let client = reqwest::ClientBuilder::new().build()?;
    let resp = client
        .get(format!("{VATSIM_OAUTH_URL_BASE}api/user"))
        .header("Authorization", &format!("Bearer {}", access_token))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Got status code {} from VATSIM OAuth user info",
            resp.status().as_u16()
        ));
    }
    let data = resp.json().await?;
    Ok(data)
}

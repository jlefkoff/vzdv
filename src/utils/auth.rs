use crate::shared::Config;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;

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
    pub vatsim: UserInfoResponseDataVatsim,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponseDataPersonal {
    pub name_first: String,
    pub name_last: String,
    pub name_full: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponseDataVatsim {
    pub rating: IdLongShort,
    #[serde(rename(deserialize = "pilotrating"))]
    pub pilot_rating: IdLongShort,
    pub division: IdName,
    pub region: IdName,
    pub subdivision: IdName,
}

#[derive(Debug, Deserialize)]
pub struct IdLongShort {
    pub id: u32,
    pub long: String,
    pub short: String,
}

#[derive(Debug, Deserialize)]
pub struct IdName {
    pub id: String,
    pub name: String,
}

/// Build the URL to redirect users to in order to start
/// their VATSIM OAuth login flow.
pub fn oauth_redirect_start(config: &Config) -> String {
    format!(
        "{}oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}",
        config.vatsim.oauth_url_base,
        config.vatsim.oauth_client_id,
        config.vatsim.oauth_client_callback_url,
        "full_name email vatsim_details"
    )
}

/// Exchange the code from VATSIM OAuth for an access token.
pub async fn code_to_user_info(code: &str, config: &Config) -> Result<TokenResponse> {
    let client = reqwest::ClientBuilder::new().build()?;
    let resp = client
        .post(format!("{}oauth/token", config.vatsim.oauth_url_base))
        .json(&json!({
            "grant_type": "authorization_code",
            "client_id": config.vatsim.oauth_client_id,
            "client_secret": config.vatsim.oauth_client_secret,
            "redirect_uri": config.vatsim.oauth_client_callback_url,
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
pub async fn get_user_info(access_token: &str, config: &Config) -> Result<UserInfoResponse> {
    let client = reqwest::ClientBuilder::new().build()?;
    let resp = client
        .get(format!("{}api/user", config.vatsim.oauth_url_base))
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

pub mod auth;
pub mod client;
pub mod factors;
pub mod login;
pub mod response;
pub mod verification;

use crate::config::app::AppProfile;
use crate::credentials::aws::AwsCredentials;
use crate::credentials::config::ConfigCredentials;
use crate::credentials::Credential;
use crate::providers::okta::client::Client;
use crate::providers::okta::login::LoginRequest;
use crate::saml;

use anyhow::{Context, Result};

const API_AUTHN_PATH: &str = "api/v1/authn";

pub struct OktaProvider {
    client: Client,
    profile: AppProfile,
}

impl OktaProvider {
    pub fn new(profile: &AppProfile) -> Result<OktaProvider> {
        Ok(OktaProvider {
            client: Client::new(profile.clone())?,
            profile: profile.clone(),
        })
    }

    pub fn new_session(&mut self) -> Result<&Self> {
        let profile = &self.profile;
        let mut config_credentials = ConfigCredentials::load(profile).unwrap_or_default();

        if !config_credentials.valid() {
            config_credentials = config_credentials.ask_password(profile)?
        };

        let username = profile.username.to_owned();
        let password = config_credentials.password.to_owned().unwrap_or_default();
        let login_response = self
            .client
            .login(&LoginRequest::from_credentials(username, password))
            .with_context(|| "Unable to login")?;

        trace!("Login response: {:?}", login_response);

        let session_token = self.client.get_session_token(login_response)?;

        config_credentials.write(profile)?;

        self.client.session_token = Some(session_token);
        Ok(self)
    }

    pub fn fetch_aws_credentials(&self) -> Result<AwsCredentials> {
        let profile = &self.profile;
        debug!("Requesting temporary STS credentials for {}", &profile.name);

        let url = profile.clone().request_url().unwrap();
        let input = self
            .client
            .get(url)
            .with_context(|| format!("Error getting SAML response for profile {}", profile.name))?
            .text()?;

        debug!("Text for SAML response: {:#?}", input);

        let credentials = saml::get_credentials_from_saml(input)?;
        trace!("Credentials: {:?}", credentials);
        Ok(credentials)
    }
}

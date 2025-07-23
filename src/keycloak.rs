use anyhow::{Context, Result};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
};
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType, CoreTokenResponse},
    reqwest::async_http_client,
    AuthenticationFlow, ClientId as OidcClientId, ClientSecret as OidcClientSecret,
    IssuerUrl, Nonce, RedirectUrl as OidcRedirectUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeycloakConfig {
    pub server_url: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl Default for KeycloakConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            realm: "master".to_string(),
            client_id: "webdriver-mcp".to_string(),
            client_secret: Some("your-client-secret".to_string()),
            redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
        }
    }
}

#[derive(Clone, Debug)]
pub struct KeycloakAuthState {
    pub csrf_token: String,
    pub pkce_verifier: Option<String>,
    pub nonce: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct KeycloakClient {
    oidc_client: CoreClient,
    #[allow(dead_code)]
    oauth2_client: BasicClient,
    config: KeycloakConfig,
}

impl KeycloakClient {
    pub async fn new(config: KeycloakConfig) -> Result<Self> {
        // Construct Keycloak URLs
        let issuer_url = format!("{}/realms/{}", config.server_url, config.realm);
        let auth_url = format!("{issuer_url}/protocol/openid-connect/auth");
        let token_url = format!("{issuer_url}/protocol/openid-connect/token");

        // Create OpenID Connect client for discovery
        let issuer = IssuerUrl::new(issuer_url).context("Invalid issuer URL")?;
        
        let provider_metadata = CoreProviderMetadata::discover_async(issuer, async_http_client)
            .await
            .context("Failed to discover Keycloak provider metadata")?;

        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            OidcClientId::new(config.client_id.clone()),
            config.client_secret.as_ref().map(|s| OidcClientSecret::new(s.clone())),
        )
        .set_redirect_uri(
            OidcRedirectUrl::new(config.redirect_uri.clone())
                .context("Invalid redirect URI")?
        );

        // Create OAuth2 client for token exchange
        let oauth2_client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            config.client_secret.as_ref().map(|s| ClientSecret::new(s.clone())),
            AuthUrl::new(auth_url).context("Invalid auth URL")?,
            Some(TokenUrl::new(token_url).context("Invalid token URL")?),
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.clone())
                .context("Invalid redirect URI")?
        );

        Ok(Self {
            oidc_client,
            oauth2_client,
            config,
        })
    }

    /// Generate authorization URL with PKCE and state
    pub fn get_authorization_url(&self) -> Result<(url::Url, KeycloakAuthState)> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let csrf_token = CsrfToken::new_random();
        let nonce = Nonce::new_random();

        let csrf_token_clone = csrf_token.clone();
        let nonce_clone = nonce.clone();
        let mut auth_request = self.oidc_client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                move || csrf_token_clone.clone(),
                move || nonce_clone.clone(),
            )
            .set_pkce_challenge(pkce_challenge);

        // Add requested scopes
        for scope in &self.config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_state, nonce_value) = auth_request.url();

        let auth_state = KeycloakAuthState {
            csrf_token: csrf_state.secret().clone(),
            pkce_verifier: Some(pkce_verifier.secret().clone()),
            nonce: Some(nonce_value.secret().clone()),
            created_at: chrono::Utc::now(),
        };

        Ok((auth_url, auth_state))
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code(
        &self,
        code: &str,
        state: &KeycloakAuthState,
    ) -> Result<CoreTokenResponse> {
        let mut token_request = self.oidc_client
            .exchange_code(AuthorizationCode::new(code.to_string()));

        // Add PKCE verifier if available
        if let Some(pkce_verifier) = &state.pkce_verifier {
            token_request = token_request.set_pkce_verifier(
                PkceCodeVerifier::new(pkce_verifier.clone())
            );
        }

        let token_response = token_request
            .request_async(async_http_client)
            .await
            .context("Failed to exchange authorization code for token")?;

        Ok(token_response)
    }

    /// Validate and extract user info from token
    pub async fn get_user_info(&self, access_token: &str) -> Result<HashMap<String, serde_json::Value>> {
        let userinfo_url = format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            self.config.server_url, self.config.realm
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch user info")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch user info: HTTP {}",
                response.status()
            ));
        }

        let user_info: HashMap<String, serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse user info response")?;

        Ok(user_info)
    }

    /// Validate access token by introspection
    pub async fn introspect_token(&self, token: &str) -> Result<TokenIntrospectionResponse> {
        let introspect_url = format!(
            "{}/realms/{}/protocol/openid-connect/token/introspect",
            self.config.server_url, self.config.realm
        );

        let mut params = vec![("token", token)];
        
        if let Some(client_secret) = &self.config.client_secret {
            params.push(("client_id", &self.config.client_id));
            params.push(("client_secret", client_secret));
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&introspect_url)
            .form(&params)
            .send()
            .await
            .context("Failed to introspect token")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Token introspection failed: HTTP {}",
                response.status()
            ));
        }

        let introspection: TokenIntrospectionResponse = response
            .json()
            .await
            .context("Failed to parse introspection response")?;

        Ok(introspection)
    }

    pub fn get_config(&self) -> &KeycloakConfig {
        &self.config
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenIntrospectionResponse {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub exp: Option<u64>,
    pub iat: Option<u64>,
    pub sub: Option<String>,
    pub aud: Option<serde_json::Value>,
    pub iss: Option<String>,
    pub token_type: Option<String>,
}

impl TokenIntrospectionResponse {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.exp {
            let now = chrono::Utc::now().timestamp() as u64;
            exp < now
        } else {
            false
        }
    }
}
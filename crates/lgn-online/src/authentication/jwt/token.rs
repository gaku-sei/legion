use std::ops::Deref;

use anyhow::{anyhow, Context};

use super::{signature_validation::SignatureValidation, Header, Validation};

/// A JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    raw: &'a str,
    signed: &'a str,
    header: &'a str,
    payload: &'a str,
    signature: &'a str,
}

impl<'a> Token<'a> {
    /// Get the header of the JWT.
    ///
    /// This may fail if the header is not valid.
    pub fn header(&self) -> anyhow::Result<Header> {
        self.header.parse()
    }

    // Get the signature of the JWT.
    //
    // This may fail if the signature is not valid.
    pub fn signature(&self) -> anyhow::Result<Vec<u8>> {
        base64::decode_config(self.signature, base64::URL_SAFE_NO_PAD)
            .context("invalid base64 JWT signature")
    }

    /// Convert the token into its claims.
    ///
    /// This method does not validate the token and should **NOT** be used most
    /// of the time.
    pub fn into_claims_unsafe<C>(self) -> anyhow::Result<C>
    where
        C: serde::de::DeserializeOwned,
    {
        self.to_claims_unsafe()
    }

    /// Parse the token into claims.
    ///
    /// This method does not validate the token and should **NOT** be used most
    /// of the time.
    pub(crate) fn to_claims_unsafe<C>(&self) -> anyhow::Result<C>
    where
        C: serde::de::DeserializeOwned,
    {
        let payload = base64::decode_config(self.payload, base64::URL_SAFE_NO_PAD)
            .context("failed to decode base64 JWT payload")?;

        serde_json::from_slice(payload.as_slice())
            .context("failed to convert JWT payload into claims")
            .map_err(Into::into)
    }

    /// Convert the token into its claims.
    ///
    /// This method validates the token and is recommended.
    pub fn into_claims<C, T>(self, validation: &Validation<'_, T>) -> anyhow::Result<C>
    where
        C: serde::de::DeserializeOwned,
        T: SignatureValidation,
    {
        self.validate(validation)?.into_claims_unsafe()
    }

    /// Validate the token without consuming it.
    pub fn validate<T>(self, validation: &Validation<'_, T>) -> anyhow::Result<Self>
    where
        T: SignatureValidation,
    {
        validation.validate_signature(&self)?;
        validation.validate_claims(&self)?;

        Ok(self)
    }

    /// Validate the signature on the token without consuming it.
    pub fn validate_signature<T>(&self, signature_validation: &T) -> anyhow::Result<()>
    where
        T: SignatureValidation,
    {
        let header = self.header()?;
        let signature = self.signature()?;

        signature_validation
            .validate_signature(&header.alg, header.kid.as_deref(), self.signed, &signature)
            .ok()
    }

    pub fn as_str(&self) -> &str {
        self.raw
    }
}

impl<'a> TryFrom<&'a str> for Token<'a> {
    type Error = anyhow::Error;

    /// Parses a JWT from a string.
    ///
    /// The string must be in the format `<header>.<payload>.<signature>`.
    ///
    /// The header, payload and signature are base64 encoded, as per RFC 7519.
    ///
    /// Note: The signature is **NOT** verified.
    ///
    /// # Example
    /// ```
    /// use lgn_online::authentication::jwt::Token;
    ///
    /// let raw_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    /// let token: Token = raw_token.clone().try_into().unwrap();
    ///
    /// assert_eq!(
    ///     token.as_str(),
    ///     raw_token,
    /// );
    /// ```
    fn try_from(raw: &'a str) -> Result<Self, Self::Error> {
        let (signed, signature) = raw.rsplit_once('.').ok_or_else(|| anyhow!("invalid JWT"))?;
        let mut parts = signed.splitn(2, '.');
        let header = parts.next().ok_or_else(|| anyhow!("missing header"))?;
        let payload = parts.next().ok_or_else(|| anyhow!("missing payload"))?;

        Ok(Self {
            raw,
            signed,
            header,
            payload,
            signature,
        })
    }
}

impl<'a> From<Token<'a>> for &'a str {
    fn from(token: Token<'a>) -> Self {
        token.raw
    }
}

impl Deref for Token<'_> {
    type Target = str;

    fn deref(&self) -> &str {
        self.raw
    }
}
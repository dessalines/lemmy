use activitystreams::unparsed::UnparsedMutExt;
use activitystreams_ext::UnparsedExtension;
use actix_web::HttpRequest;
use anyhow::anyhow;
use http::{header::HeaderName, HeaderMap, HeaderValue};
use http_signature_normalization_actix::Config as ConfigActix;
use http_signature_normalization_reqwest::prelude::{Config, SignExt};
use lemmy_utils::LemmyError;
use log::debug;
use openssl::{
  hash::MessageDigest,
  pkey::PKey,
  sign::{Signer, Verifier},
};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, str::FromStr};
use url::Url;

lazy_static! {
  static ref CONFIG2: ConfigActix = ConfigActix::new();
  static ref HTTP_SIG_CONFIG: Config = Config::new();
}

/// Creates an HTTP post request to `inbox_url`, with the given `client` and `headers`, and
/// `activity` as request body. The request is signed with `private_key` and then sent.
pub(crate) async fn sign_and_send(
  client: &Client,
  headers: BTreeMap<String, String>,
  inbox_url: &Url,
  activity: String,
  actor_id: &Url,
  private_key: String,
) -> Result<Response, LemmyError> {
  let signing_key_id = format!("{}#main-key", actor_id);

  let mut header_map = HeaderMap::new();
  for h in headers {
    header_map.insert(
      HeaderName::from_str(h.0.as_str())?,
      HeaderValue::from_str(h.1.as_str())?,
    );
  }
  let response = client
    .post(&inbox_url.to_string())
    .headers(header_map)
    .signature_with_digest(
      HTTP_SIG_CONFIG.clone(),
      signing_key_id,
      Sha256::new(),
      activity,
      move |signing_string| {
        let private_key = PKey::private_key_from_pem(private_key.as_bytes())?;
        let mut signer = Signer::new(MessageDigest::sha256(), &private_key)?;
        signer.update(signing_string.as_bytes())?;

        Ok(base64::encode(signer.sign_to_vec()?)) as Result<_, LemmyError>
      },
    )
    .await?;

  Ok(response)
}

/// Verifies the HTTP signature on an incoming inbox request.
pub fn verify_signature(request: &HttpRequest, public_key: &str) -> Result<(), LemmyError> {
  let verified = CONFIG2
    .begin_verify(
      request.method(),
      request.uri().path_and_query(),
      request.headers().clone(),
    )?
    .verify(|signature, signing_string| -> Result<bool, LemmyError> {
      debug!(
        "Verifying with key {}, message {}",
        &public_key, &signing_string
      );
      let public_key = PKey::public_key_from_pem(public_key.as_bytes())?;
      let mut verifier = Verifier::new(MessageDigest::sha256(), &public_key)?;
      verifier.update(signing_string.as_bytes())?;
      Ok(verifier.verify(&base64::decode(signature)?)?)
    })?;

  if verified {
    debug!("verified signature for {}", &request.uri());
    Ok(())
  } else {
    Err(anyhow!("Invalid signature on request: {}", &request.uri()).into())
  }
}

/// Extension for actor public key, which is needed on person and community for HTTP signatures.
///
/// Taken from: https://docs.rs/activitystreams/0.5.0-alpha.17/activitystreams/ext/index.html
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyExtension {
  pub public_key: PublicKey,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
  pub id: String,
  pub owner: Url,
  pub public_key_pem: String,
}

impl PublicKey {
  pub fn to_ext(&self) -> PublicKeyExtension {
    PublicKeyExtension {
      public_key: self.to_owned(),
    }
  }
}

impl<U> UnparsedExtension<U> for PublicKeyExtension
where
  U: UnparsedMutExt,
{
  type Error = serde_json::Error;

  fn try_from_unparsed(unparsed_mut: &mut U) -> Result<Self, Self::Error> {
    Ok(PublicKeyExtension {
      public_key: unparsed_mut.remove("publicKey")?,
    })
  }

  fn try_into_unparsed(self, unparsed_mut: &mut U) -> Result<(), Self::Error> {
    unparsed_mut.insert("publicKey", self.public_key)?;
    Ok(())
  }
}

use crate::activities::{
  following::accept::AcceptFollowCommunity,
  post::{create::CreatePost, like::LikePost},
};
use actix_web::{
  body::Body,
  web,
  web::{Bytes, BytesMut, Payload},
  HttpRequest,
  HttpResponse,
};
use anyhow::{anyhow, Context};
use futures::StreamExt;
use http::StatusCode;
use lemmy_api_common::blocking;
use lemmy_apub::{
  check_is_apub_id_valid,
  extensions::signatures::verify_signature,
  fetcher::get_or_fetch_and_upsert_actor,
  insert_activity,
  APUB_JSON_CONTENT_TYPE,
};
use lemmy_apub_lib::{ActivityCommonFields, ActivityHandlerNew};
use lemmy_db_queries::{source::activity::Activity_, DbPool};
use lemmy_db_schema::source::activity::Activity;
use lemmy_utils::{location_info, settings::structs::Settings, LemmyError};
use lemmy_websocket::LemmyContext;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, io::Read};
use url::Url;

pub mod comment;
pub mod community;
pub mod inbox_enums;
pub mod person;
pub mod post;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ActivityHandlerNew)]
#[serde(untagged)]
enum Ac {
  CreatePost(CreatePost),
  LikePost(LikePost),
  AcceptFollowCommunity(AcceptFollowCommunity),
}

pub async fn shared_inbox(
  request: HttpRequest,
  payload: Payload,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse, LemmyError> {
  let unparsed = payload_to_string(payload).await?;
  receive_activity::<Ac>(request, &unparsed, context).await
}

async fn payload_to_string(mut payload: Payload) -> Result<String, LemmyError> {
  let mut bytes = BytesMut::new();
  while let Some(item) = payload.next().await {
    bytes.extend_from_slice(&item?);
  }
  let mut unparsed = String::new();
  Bytes::from(bytes).as_ref().read_to_string(&mut unparsed)?;
  Ok(unparsed)
}
async fn receive_activity<'a, T>(
  request: HttpRequest,
  activity: &'a str,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse, LemmyError>
where
  T: ActivityHandlerNew + Clone + Deserialize<'a> + Serialize + std::fmt::Debug + Send + 'static,
{
  debug!("Received activity {}", activity);
  let activity = serde_json::from_str::<T>(activity)?;
  let activity_data = activity.common();
  // TODO: which order to check things?
  // Do nothing if we received the same activity before
  if is_activity_already_known(context.pool(), activity_data.id_unchecked()).await? {
    return Ok(HttpResponse::Ok().finish());
  }
  assert_activity_not_local(&activity)?;
  check_is_apub_id_valid(&activity_data.actor, false)?;

  let request_counter = &mut 0;
  let actor =
    get_or_fetch_and_upsert_actor(&activity_data.actor, &context, request_counter).await?;
  verify_signature(&request, &actor.public_key().context(location_info!())?)?;
  activity.verify(&context, request_counter).await?;

  // Log the activity, so we avoid receiving and parsing it twice. Note that this could still happen
  // if we receive the same activity twice in very quick succession.
  insert_activity(
    activity_data.id_unchecked(),
    activity.clone(),
    false,
    true,
    context.pool(),
  )
  .await?;

  activity.receive(&context, request_counter).await?;
  Ok(HttpResponse::Ok().finish())
}

/// Convert the data to json and turn it into an HTTP Response with the correct ActivityPub
/// headers.
fn create_apub_response<T>(data: &T) -> HttpResponse<Body>
where
  T: Serialize,
{
  HttpResponse::Ok()
    .content_type(APUB_JSON_CONTENT_TYPE)
    .json(data)
}

fn create_apub_tombstone_response<T>(data: &T) -> HttpResponse<Body>
where
  T: Serialize,
{
  HttpResponse::Gone()
    .content_type(APUB_JSON_CONTENT_TYPE)
    .status(StatusCode::GONE)
    .json(data)
}

#[derive(Deserialize)]
pub struct ActivityQuery {
  type_: String,
  id: String,
}

/// Return the ActivityPub json representation of a local activity over HTTP.
pub(crate) async fn get_activity(
  info: web::Path<ActivityQuery>,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse<Body>, LemmyError> {
  let settings = Settings::get();
  let activity_id = Url::parse(&format!(
    "{}/activities/{}/{}",
    settings.get_protocol_and_hostname(),
    info.type_,
    info.id
  ))?
  .into();
  let activity = blocking(context.pool(), move |conn| {
    Activity::read_from_apub_id(conn, &activity_id)
  })
  .await??;

  let sensitive = activity.sensitive.unwrap_or(true);
  if !activity.local || sensitive {
    Ok(HttpResponse::NotFound().finish())
  } else {
    Ok(create_apub_response(&activity.data))
  }
}

pub(crate) async fn is_activity_already_known(
  pool: &DbPool,
  activity_id: &Url,
) -> Result<bool, LemmyError> {
  let activity_id = activity_id.to_owned().into();
  let existing = blocking(pool, move |conn| {
    Activity::read_from_apub_id(conn, &activity_id)
  })
  .await?;
  match existing {
    Ok(_) => Ok(true),
    Err(_) => Ok(false),
  }
}

fn assert_activity_not_local<T: Debug + ActivityHandlerNew>(
  activity: &T,
) -> Result<(), LemmyError> {
  let activity_domain = activity
    .common()
    .id_unchecked()
    .domain()
    .context(location_info!())?;

  if activity_domain == Settings::get().hostname() {
    return Err(
      anyhow!(
        "Error: received activity which was sent by local instance: {:?}",
        activity
      )
      .into(),
    );
  }
  Ok(())
}

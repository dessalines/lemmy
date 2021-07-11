use crate::http::{
  create_apub_response,
  create_apub_tombstone_response,
  inbox_enums::PersonInboxActivities,
  payload_to_string,
  receive_activity,
};
use activitystreams::{
  base::BaseExt,
  collection::{CollectionExt, OrderedCollection},
};
use actix_web::{body::Body, web, web::Payload, HttpRequest, HttpResponse};
use lemmy_api_common::blocking;
use lemmy_apub::{extensions::context::lemmy_context, objects::ToApub, ActorType};
use lemmy_db_queries::source::person::Person_;
use lemmy_db_schema::source::person::Person;
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
pub struct PersonQuery {
  user_name: String,
}

/// Return the ActivityPub json representation of a local person over HTTP.
pub(crate) async fn get_apub_person_http(
  info: web::Path<PersonQuery>,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse<Body>, LemmyError> {
  let user_name = info.into_inner().user_name;
  // TODO: this needs to be able to read deleted persons, so that it can send tombstones
  let person = blocking(context.pool(), move |conn| {
    Person::find_by_name(conn, &user_name)
  })
  .await??;

  if !person.deleted {
    let apub = person.to_apub(context.pool()).await?;

    Ok(create_apub_response(&apub))
  } else {
    Ok(create_apub_tombstone_response(&person.to_tombstone()?))
  }
}

pub async fn person_inbox(
  request: HttpRequest,
  payload: Payload,
  _path: web::Path<String>,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse, LemmyError> {
  let unparsed = payload_to_string(payload).await?;
  receive_activity::<PersonInboxActivities>(request, &unparsed, context).await
}

pub(crate) async fn get_apub_person_outbox(
  info: web::Path<PersonQuery>,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse<Body>, LemmyError> {
  let person = blocking(context.pool(), move |conn| {
    Person::find_by_name(conn, &info.user_name)
  })
  .await??;
  // TODO: populate the person outbox
  let mut collection = OrderedCollection::new();
  collection
    .set_many_items(Vec::<Url>::new())
    .set_many_contexts(lemmy_context()?)
    .set_id(person.get_outbox_url()?)
    .set_total_items(0_u64);
  Ok(create_apub_response(&collection))
}

pub(crate) async fn get_apub_person_inbox(
  info: web::Path<PersonQuery>,
  context: web::Data<LemmyContext>,
) -> Result<HttpResponse<Body>, LemmyError> {
  let person = blocking(context.pool(), move |conn| {
    Person::find_by_name(conn, &info.user_name)
  })
  .await??;

  let mut collection = OrderedCollection::new();
  collection
    .set_id(person.inbox_url.into())
    .set_many_contexts(lemmy_context()?);
  Ok(create_apub_response(&collection))
}

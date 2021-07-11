use crate::activities::post::send_websocket_message;
use activitystreams::{activity::kind::CreateType, base::BaseExt};
use lemmy_apub::{
  check_is_apub_id_valid,
  fetcher::person::get_or_fetch_and_upsert_person,
  objects::FromApub,
  ActorType,
  PageExt,
};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_db_schema::source::post::Post;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePost {
  to: PublicUrl,
  object: PageExt,
  cc: Vec<Url>,
  #[serde(rename = "type")]
  kind: CreateType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for CreatePost {
  async fn verify(&self, _context: &LemmyContext, _: &mut i32) -> Result<(), LemmyError> {
    verify_domains_match(self.common.id_unchecked(), &self.common.actor)?;
    self.object.id(self.common.actor.as_str())?;
    check_is_apub_id_valid(&self.common.actor, false)
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let actor =
      get_or_fetch_and_upsert_person(&self.common.actor, context, request_counter).await?;
    let post = Post::from_apub(
      &self.object,
      context,
      actor.actor_id(),
      request_counter,
      false,
    )
    .await?;

    send_websocket_message(post.id, UserOperationCrud::CreatePost, context).await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

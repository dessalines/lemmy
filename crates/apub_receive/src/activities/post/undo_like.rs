use crate::activities::post::{like::LikePost, undo_like_or_dislike_post};
use activitystreams::activity::kind::UndoType;
use lemmy_apub::check_is_apub_id_valid;
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoLikePost {
  to: PublicUrl,
  object: LikePost,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: UndoType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}
#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UndoLikePost {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    verify_domains_match(&self.common.actor, &self.object.object)?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    self.object.verify(context, request_counter).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    undo_like_or_dislike_post(
      &self.common.actor,
      &self.object.object,
      context,
      request_counter,
    )
    .await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

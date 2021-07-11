use crate::activities::comment::{get_notif_recipients, send_websocket_message};
use activitystreams::{activity::kind::CreateType, base::BaseExt};
use lemmy_apub::{check_is_apub_id_valid, objects::FromApub, NoteExt};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_db_schema::source::comment::Comment;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateComment {
  to: PublicUrl,
  object: NoteExt,
  cc: Vec<Url>,
  #[serde(rename = "type")]
  kind: CreateType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for CreateComment {
  async fn verify(&self, _context: &LemmyContext, _: &mut i32) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    self.object.id(self.common.actor.as_str())?;
    check_is_apub_id_valid(&self.common.actor, false)
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let comment = Comment::from_apub(
      &self.object,
      context,
      self.common.actor.clone(),
      request_counter,
      false,
    )
    .await?;
    let recipients =
      get_notif_recipients(&self.common.actor, &comment, context, request_counter).await?;
    send_websocket_message(
      comment.id,
      recipients,
      UserOperationCrud::CreateComment,
      context,
    )
    .await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

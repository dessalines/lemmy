use crate::activities::private_message::{send_websocket_message, verify_activity, verify_person};
use activitystreams::{activity::kind::UpdateType, base::BaseExt};
use lemmy_apub::{objects::FromApub, NoteExt};
use lemmy_apub_lib::{verify_domains_match_opt, ActivityCommonFields, ActivityHandlerNew};
use lemmy_db_schema::source::private_message::PrivateMessage;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrivateMessage {
  to: Url,
  object: NoteExt,
  #[serde(rename = "type")]
  kind: UpdateType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UpdatePrivateMessage {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_activity(self.common())?;
    verify_person(&self.common.actor, context, request_counter).await?;
    verify_domains_match_opt(&self.common.actor, self.object.id_unchecked())?;
    Ok(())
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let private_message = PrivateMessage::from_apub(
      &self.object,
      context,
      self.common.actor.clone(),
      request_counter,
      false,
    )
    .await?;

    send_websocket_message(
      private_message.id,
      UserOperationCrud::EditPrivateMessage,
      context,
    )
    .await?;

    Ok(())
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

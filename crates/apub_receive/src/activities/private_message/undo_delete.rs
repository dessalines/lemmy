use crate::activities::private_message::{
  delete::DeletePrivateMessage,
  send_websocket_message,
  verify_activity,
  verify_person,
};
use activitystreams::activity::kind::UndoType;
use lemmy_api_common::blocking;
use lemmy_apub_lib::{
  verify_domains_match,
  verify_urls_match,
  ActivityCommonFields,
  ActivityHandlerNew,
};
use lemmy_db_queries::{source::private_message::PrivateMessage_, ApubObject};
use lemmy_db_schema::source::private_message::PrivateMessage;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoDeletePrivateMessage {
  to: Url,
  object: DeletePrivateMessage,
  #[serde(rename = "type")]
  kind: UndoType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UndoDeletePrivateMessage {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_activity(self.common())?;
    verify_person(&self.common.actor, context, request_counter).await?;
    verify_urls_match(&self.common.actor, &self.object.common.actor)?;
    verify_domains_match(&self.common.actor, &self.object.object)?;
    self.object.verify(context, request_counter).await?;
    Ok(())
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    _request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let ap_id = self.object.object.clone();
    let private_message = blocking(context.pool(), move |conn| {
      PrivateMessage::read_from_apub_id(conn, &ap_id.into())
    })
    .await??;

    let deleted_private_message = blocking(context.pool(), move |conn| {
      PrivateMessage::update_deleted(conn, private_message.id, false)
    })
    .await??;

    send_websocket_message(
      deleted_private_message.id,
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

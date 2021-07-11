use crate::activities::private_message::{send_websocket_message, verify_activity, verify_person};
use activitystreams::activity::kind::DeleteType;
use lemmy_api_common::blocking;
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew};
use lemmy_db_queries::{source::private_message::PrivateMessage_, ApubObject};
use lemmy_db_schema::source::private_message::PrivateMessage;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePrivateMessage {
  to: Url,
  pub(in crate::activities::private_message) object: Url,
  #[serde(rename = "type")]
  kind: DeleteType,
  #[serde(flatten)]
  pub(in crate::activities::private_message) common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for DeletePrivateMessage {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_activity(self.common())?;
    verify_person(&self.common.actor, context, request_counter).await?;
    verify_domains_match(&self.common.actor, &self.object)?;
    Ok(())
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    _request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let ap_id = self.object.clone();
    let private_message = blocking(context.pool(), move |conn| {
      PrivateMessage::read_from_apub_id(conn, &ap_id.into())
    })
    .await??;
    let deleted_private_message = blocking(context.pool(), move |conn| {
      PrivateMessage::update_deleted(conn, private_message.id, true)
    })
    .await??;

    send_websocket_message(
      deleted_private_message.id,
      UserOperationCrud::DeletePrivateMessage,
      context,
    )
    .await?;

    Ok(())
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

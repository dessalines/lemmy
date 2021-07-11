use crate::activities::comment::{delete::DeleteComment, send_websocket_message};
use activitystreams::activity::kind::UndoType;
use lemmy_api_common::blocking;
use lemmy_apub::{check_is_apub_id_valid, fetcher::objects::get_or_fetch_and_insert_comment};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_db_queries::source::comment::Comment_;
use lemmy_db_schema::source::comment::Comment;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoDeleteComment {
  to: PublicUrl,
  object: DeleteComment,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: UndoType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UndoDeleteComment {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    verify_domains_match(&self.common.actor, &self.object.common.actor)?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    self.object.verify(context, request_counter).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let comment =
      get_or_fetch_and_insert_comment(&self.object.object, context, request_counter).await?;

    let deleted_comment = blocking(context.pool(), move |conn| {
      Comment::update_deleted(conn, comment.id, false)
    })
    .await??;

    send_websocket_message(
      deleted_comment.id,
      vec![],
      UserOperationCrud::EditComment,
      context,
    )
    .await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

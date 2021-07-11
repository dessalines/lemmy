use crate::activities::{
  comment::{remove::RemoveComment, send_websocket_message},
  verify_mod_action,
};
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
pub struct UndoRemoveComment {
  to: PublicUrl,
  object: RemoveComment,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: UndoType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UndoRemoveComment {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    verify_mod_action(self.common.actor.clone(), self.cc[0].clone(), context).await?;
    self.object.verify(context, request_counter).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let comment =
      get_or_fetch_and_insert_comment(&self.object.object, context, request_counter).await?;

    let removed_comment = blocking(context.pool(), move |conn| {
      Comment::update_removed(conn, comment.id, false)
    })
    .await??;

    send_websocket_message(
      removed_comment.id,
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

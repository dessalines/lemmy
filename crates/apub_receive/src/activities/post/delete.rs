use crate::activities::post::send_websocket_message;
use activitystreams::activity::kind::DeleteType;
use lemmy_api_common::blocking;
use lemmy_apub::{check_is_apub_id_valid, fetcher::objects::get_or_fetch_and_insert_post};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_db_queries::source::post::Post_;
use lemmy_db_schema::source::post::Post;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePost {
  to: PublicUrl,
  pub(in crate::activities::post) object: Url,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: DeleteType,
  #[serde(flatten)]
  pub(in crate::activities::post) common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for DeletePost {
  async fn verify(&self, _context: &LemmyContext, _: &mut i32) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    verify_domains_match(&self.common.actor, &self.object)?;
    check_is_apub_id_valid(&self.common.actor, false)
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let post = get_or_fetch_and_insert_post(&self.object, context, request_counter).await?;

    let deleted_post = blocking(context.pool(), move |conn| {
      Post::update_deleted(conn, post.id, true)
    })
    .await??;

    send_websocket_message(deleted_post.id, UserOperationCrud::EditPost, context).await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

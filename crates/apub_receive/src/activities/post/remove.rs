use crate::activities::{post::send_websocket_message, verify_mod_action};
use activitystreams::activity::kind::RemoveType;
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
pub struct RemovePost {
  to: PublicUrl,
  pub(in crate::activities::post) object: Url,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: RemoveType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for RemovePost {
  async fn verify(&self, context: &LemmyContext, _: &mut i32) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    verify_mod_action(self.common.actor.clone(), self.cc[0].clone(), context).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    // TODO: check that actor is instance mod if community is local (same for undo, RemoveComment)
    let post = get_or_fetch_and_insert_post(&self.object, context, request_counter).await?;

    let removed_post = blocking(context.pool(), move |conn| {
      Post::update_removed(conn, post.id, true)
    })
    .await??;

    send_websocket_message(removed_post.id, UserOperationCrud::EditPost, context).await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

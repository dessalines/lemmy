use crate::activities::comment::like_or_dislike_comment;
use activitystreams::activity::kind::LikeType;
use lemmy_apub::check_is_apub_id_valid;
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LikeComment {
  to: PublicUrl,
  pub(in crate::activities::comment) object: Url,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: LikeType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for LikeComment {
  async fn verify(&self, _context: &LemmyContext, _: &mut i32) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    check_is_apub_id_valid(&self.common.actor, false)
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    like_or_dislike_comment(
      -1,
      &self.common.actor,
      &self.object,
      context,
      request_counter,
    )
    .await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

use crate::activities::following::follow::FollowCommunity;
use activitystreams::activity::kind::AcceptType;
use lemmy_api_common::blocking;
use lemmy_apub::{
  check_is_apub_id_valid,
  fetcher::{community::get_or_fetch_and_upsert_community, person::get_or_fetch_and_upsert_person},
};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew};
use lemmy_db_queries::Followable;
use lemmy_db_schema::source::community::CommunityFollower;
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptFollowCommunity {
  to: Url,
  object: FollowCommunity,
  #[serde(rename = "type")]
  kind: AcceptType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

/// Handle accepted follows
#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for AcceptFollowCommunity {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    self.object.verify(context, request_counter).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let actor =
      get_or_fetch_and_upsert_community(&self.common.actor, context, request_counter).await?;
    let to = get_or_fetch_and_upsert_person(&self.to, context, request_counter).await?;
    // This will throw an error if no follow was requested
    blocking(context.pool(), move |conn| {
      CommunityFollower::follow_accepted(conn, actor.id, to.id)
    })
    .await??;

    Ok(())
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

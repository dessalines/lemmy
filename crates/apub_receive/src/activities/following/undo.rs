use crate::activities::following::follow::FollowCommunity;
use activitystreams::activity::kind::UndoType;
use lemmy_api_common::blocking;
use lemmy_apub::{
  check_is_apub_id_valid,
  fetcher::{community::get_or_fetch_and_upsert_community, person::get_or_fetch_and_upsert_person},
};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew};
use lemmy_db_queries::Followable;
use lemmy_db_schema::source::community::{CommunityFollower, CommunityFollowerForm};
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoFollowCommunity {
  to: Url,
  object: FollowCommunity,
  #[serde(rename = "type")]
  kind: UndoType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UndoFollowCommunity {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    verify_domains_match(&self.to, &self.object.object)?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    self.object.verify(context, request_counter).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let actor =
      get_or_fetch_and_upsert_person(&self.common.actor, context, request_counter).await?;
    let community = get_or_fetch_and_upsert_community(&self.to, context, request_counter).await?;

    let community_follower_form = CommunityFollowerForm {
      community_id: community.id,
      person_id: actor.id,
      pending: false,
    };

    // This will fail if they aren't a follower, but ignore the error.
    blocking(context.pool(), move |conn| {
      CommunityFollower::unfollow(conn, &community_follower_form).ok()
    })
    .await?;
    Ok(())
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

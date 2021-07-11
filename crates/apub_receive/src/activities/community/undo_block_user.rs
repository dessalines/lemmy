use crate::activities::{community::block_user::BlockUserFromCommunity, verify_mod_action};
use activitystreams::activity::kind::BlockType;
use lemmy_api_common::blocking;
use lemmy_apub::{
  check_is_apub_id_valid,
  fetcher::{community::get_or_fetch_and_upsert_community, person::get_or_fetch_and_upsert_person},
};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_db_queries::Bannable;
use lemmy_db_schema::source::community::{CommunityPersonBan, CommunityPersonBanForm};
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoBlockUserFromCommunity {
  to: PublicUrl,
  object: BlockUserFromCommunity,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: BlockType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for UndoBlockUserFromCommunity {
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
    let community =
      get_or_fetch_and_upsert_community(&self.cc[0], context, request_counter).await?;
    let blocked_user =
      get_or_fetch_and_upsert_person(&self.object.object, context, request_counter).await?;

    let community_user_ban_form = CommunityPersonBanForm {
      community_id: community.id,
      person_id: blocked_user.id,
    };

    blocking(context.pool(), move |conn: &'_ _| {
      CommunityPersonBan::unban(conn, &community_user_ban_form)
    })
    .await??;

    Ok(())
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

use crate::activities::community::send_websocket_message;
use activitystreams::activity::kind::RemoveType;
use lemmy_api_common::blocking;
use lemmy_apub::check_is_apub_id_valid;
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_db_queries::{source::community::Community_, ApubObject};
use lemmy_db_schema::source::community::Community;
use lemmy_utils::LemmyError;
use lemmy_websocket::{LemmyContext, UserOperationCrud};
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveCommunity {
  to: PublicUrl,
  pub(in crate::activities::community) object: Url,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: RemoveType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for RemoveCommunity {
  async fn verify(&self, _context: &LemmyContext, _: &mut i32) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    verify_domains_match(&self.common.actor, &self.object)?;
    verify_domains_match(&self.common.actor, &self.cc[0])
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    _request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    let object = self.object.clone();
    // only search in local database, there is no reason to fetch something thats deleted
    let community = blocking(context.pool(), move |conn| {
      Community::read_from_apub_id(conn, &object.into())
    })
    .await??;
    let removed_community = blocking(context.pool(), move |conn| {
      Community::update_removed(conn, community.id, true)
    })
    .await??;

    send_websocket_message(
      removed_community.id,
      UserOperationCrud::RemoveCommunity,
      context,
    )
    .await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

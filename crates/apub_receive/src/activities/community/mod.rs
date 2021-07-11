use anyhow::anyhow;
use lemmy_api_common::{blocking, community::CommunityResponse};
use lemmy_apub::generate_moderators_url;
use lemmy_db_queries::ApubObject;
use lemmy_db_schema::{
  source::{community::Community, person::Person},
  CommunityId,
};
use lemmy_db_views_actor::community_view::CommunityView;
use lemmy_utils::LemmyError;
use lemmy_websocket::{messages::SendCommunityRoomMessage, LemmyContext};
use url::Url;

pub mod add_mod;
pub mod announce;
pub mod block_user;
pub mod delete;
pub mod remove;
pub mod remove_mod;
pub mod undo_block_user;
pub mod undo_delete;
pub mod undo_remove;
pub mod update;

async fn send_websocket_message<OP: ToString + Send + lemmy_websocket::OperationType + 'static>(
  community_id: CommunityId,
  op: OP,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  let community_view = blocking(context.pool(), move |conn| {
    CommunityView::read(conn, community_id, None)
  })
  .await??;

  let res = CommunityResponse { community_view };

  context.chat_server().do_send(SendCommunityRoomMessage {
    op,
    response: res,
    community_id,
    websocket_id: None,
  });

  Ok(())
}

// TODO: why do we have this and verify_mod_action() ?
async fn verify_is_community_mod(
  actor: Url,
  community: Url,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  let actor = blocking(context.pool(), move |conn| {
    Person::read_from_apub_id(conn, &actor.into())
  })
  .await??;
  let community = blocking(context.pool(), move |conn| {
    Community::read_from_apub_id(conn, &community.into())
  })
  .await??;
  let is_mod_or_admin = blocking(context.pool(), move |conn| {
    CommunityView::is_mod_or_admin(conn, actor.id, community.id)
  })
  .await?;
  if !is_mod_or_admin {
    return Err(anyhow!("Not a mod").into());
  }
  Ok(())
}

/// For Add/Remove community moderator activities, check that the target field actually contains
/// /c/community/moderators. Any different values are unsupported.
fn verify_add_remove_moderator_target(target: &Url, community: Url) -> Result<(), LemmyError> {
  if target != &generate_moderators_url(&community.into())?.into_inner() {
    return Err(anyhow!("Unkown target url").into());
  }
  Ok(())
}

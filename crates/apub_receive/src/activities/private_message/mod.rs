use anyhow::anyhow;
use lemmy_api_common::{blocking, person::PrivateMessageResponse};
use lemmy_apub::{check_is_apub_id_valid, fetcher::person::get_or_fetch_and_upsert_person};
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields};
use lemmy_db_schema::PrivateMessageId;
use lemmy_db_views::{local_user_view::LocalUserView, private_message_view::PrivateMessageView};
use lemmy_utils::LemmyError;
use lemmy_websocket::{messages::SendUserRoomMessage, LemmyContext, UserOperationCrud};
use url::Url;

pub mod create;
pub mod delete;
pub mod undo_delete;
pub mod update;

/// Checks that the specified Url actually identifies a Person (by fetching it), and that the person
/// doesn't have a site ban.
async fn verify_person(
  person_id: &Url,
  context: &LemmyContext,
  request_counter: &mut i32,
) -> Result<(), LemmyError> {
  let person = get_or_fetch_and_upsert_person(person_id, context, request_counter).await?;
  if person.banned {
    return Err(anyhow!("Person {} is banned", person_id).into());
  }
  Ok(())
}

fn verify_activity(common: &ActivityCommonFields) -> Result<(), LemmyError> {
  check_is_apub_id_valid(&common.actor, false)?;
  verify_domains_match(common.id_unchecked(), &common.actor)?;
  Ok(())
}

async fn send_websocket_message(
  private_message_id: PrivateMessageId,
  op: UserOperationCrud,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  let message = blocking(context.pool(), move |conn| {
    PrivateMessageView::read(conn, private_message_id)
  })
  .await??;
  let res = PrivateMessageResponse {
    private_message_view: message,
  };

  // Send notifications to the local recipient, if one exists
  let recipient_id = res.private_message_view.recipient.id;
  let local_recipient_id = blocking(context.pool(), move |conn| {
    LocalUserView::read_person(conn, recipient_id)
  })
  .await??
  .local_user
  .id;

  context.chat_server().do_send(SendUserRoomMessage {
    op,
    response: res,
    local_recipient_id,
    websocket_id: None,
  });

  Ok(())
}

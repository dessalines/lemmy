use crate::PerformCrud;
use actix_web::web::Data;
use lemmy_api_common::{
  blocking,
  check_person_block,
  get_local_user_view_from_jwt,
  person::{CreatePrivateMessage, PrivateMessageResponse},
  send_email_to_user,
};
use lemmy_apub::{generate_apub_endpoint, ApubObjectType, EndpointType};
use lemmy_db_queries::{source::private_message::PrivateMessage_, Crud};
use lemmy_db_schema::source::private_message::{PrivateMessage, PrivateMessageForm};
use lemmy_db_views::{local_user_view::LocalUserView, private_message_view::PrivateMessageView};
use lemmy_utils::{utils::remove_slurs, ApiError, ConnectionId, LemmyError};
use lemmy_websocket::{messages::SendUserRoomMessage, LemmyContext, UserOperationCrud};

#[async_trait::async_trait(?Send)]
impl PerformCrud for CreatePrivateMessage {
  type Response = PrivateMessageResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<PrivateMessageResponse, LemmyError> {
    let data: &CreatePrivateMessage = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let content_slurs_removed = remove_slurs(&data.content.to_owned());

    check_person_block(local_user_view.person.id, data.recipient_id, context.pool()).await?;

    let private_message_form = PrivateMessageForm {
      content: content_slurs_removed.to_owned(),
      creator_id: local_user_view.person.id,
      recipient_id: data.recipient_id,
      ..PrivateMessageForm::default()
    };

    let inserted_private_message = match blocking(context.pool(), move |conn| {
      PrivateMessage::create(conn, &private_message_form)
    })
    .await?
    {
      Ok(private_message) => private_message,
      Err(_e) => {
        return Err(ApiError::err("couldnt_create_private_message").into());
      }
    };

    let inserted_private_message_id = inserted_private_message.id;
    let updated_private_message = blocking(
      context.pool(),
      move |conn| -> Result<PrivateMessage, LemmyError> {
        let apub_id = generate_apub_endpoint(
          EndpointType::PrivateMessage,
          &inserted_private_message_id.to_string(),
        )?;
        Ok(PrivateMessage::update_ap_id(
          conn,
          inserted_private_message_id,
          apub_id,
        )?)
      },
    )
    .await?
    .map_err(|_| ApiError::err("couldnt_create_private_message"))?;

    updated_private_message
      .send_create(&local_user_view.person, context)
      .await?;

    let private_message_view = blocking(context.pool(), move |conn| {
      PrivateMessageView::read(conn, inserted_private_message.id)
    })
    .await??;

    let res = PrivateMessageResponse {
      private_message_view,
    };

    // Send notifications to the local recipient, if one exists
    let recipient_id = data.recipient_id;
    if let Ok(local_recipient) = blocking(context.pool(), move |conn| {
      LocalUserView::read_person(conn, recipient_id)
    })
    .await?
    {
      send_email_to_user(
        &local_recipient,
        "Private Message from",
        "Private Message",
        &content_slurs_removed,
      );

      let local_recipient_id = local_recipient.local_user.id;
      context.chat_server().do_send(SendUserRoomMessage {
        op: UserOperationCrud::CreatePrivateMessage,
        response: res.clone(),
        local_recipient_id,
        websocket_id,
      });
    }

    Ok(res)
  }
}

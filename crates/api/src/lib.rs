use actix_web::{web, web::Data};
use captcha::Captcha;
use lemmy_api_common::{comment::*, community::*, person::*, post::*, site::*, websocket::*};
use lemmy_utils::{ConnectionId, LemmyError};
use lemmy_websocket::{serialize_websocket_message, LemmyContext, UserOperation};
use serde::Deserialize;

mod comment;
mod comment_report;
mod community;
mod local_user;
mod post;
mod post_report;
mod private_message;
mod site;
mod websocket;

#[async_trait::async_trait(?Send)]
pub trait Perform {
  type Response: serde::ser::Serialize + Send;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<Self::Response, LemmyError>;
}

pub async fn match_websocket_operation(
  context: LemmyContext,
  id: ConnectionId,
  op: UserOperation,
  data: &str,
) -> Result<String, LemmyError> {
  match op {
    // User ops
    UserOperation::Login => do_websocket_operation::<Login>(context, id, op, data).await,
    UserOperation::GetCaptcha => do_websocket_operation::<GetCaptcha>(context, id, op, data).await,
    UserOperation::GetReplies => do_websocket_operation::<GetReplies>(context, id, op, data).await,
    UserOperation::AddAdmin => do_websocket_operation::<AddAdmin>(context, id, op, data).await,
    UserOperation::BanPerson => do_websocket_operation::<BanPerson>(context, id, op, data).await,
    UserOperation::BlockPerson => {
      do_websocket_operation::<BlockPerson>(context, id, op, data).await
    }
    UserOperation::GetPersonMentions => {
      do_websocket_operation::<GetPersonMentions>(context, id, op, data).await
    }
    UserOperation::GetBlockedPersons => {
      do_websocket_operation::<GetBlockedPersons>(context, id, op, data).await
    }
    UserOperation::MarkPersonMentionAsRead => {
      do_websocket_operation::<MarkPersonMentionAsRead>(context, id, op, data).await
    }
    UserOperation::MarkAllAsRead => {
      do_websocket_operation::<MarkAllAsRead>(context, id, op, data).await
    }
    UserOperation::PasswordReset => {
      do_websocket_operation::<PasswordReset>(context, id, op, data).await
    }
    UserOperation::PasswordChange => {
      do_websocket_operation::<PasswordChange>(context, id, op, data).await
    }
    UserOperation::UserJoin => do_websocket_operation::<UserJoin>(context, id, op, data).await,
    UserOperation::PostJoin => do_websocket_operation::<PostJoin>(context, id, op, data).await,
    UserOperation::CommunityJoin => {
      do_websocket_operation::<CommunityJoin>(context, id, op, data).await
    }
    UserOperation::ModJoin => do_websocket_operation::<ModJoin>(context, id, op, data).await,
    UserOperation::SaveUserSettings => {
      do_websocket_operation::<SaveUserSettings>(context, id, op, data).await
    }
    UserOperation::ChangePassword => {
      do_websocket_operation::<ChangePassword>(context, id, op, data).await
    }
    UserOperation::GetReportCount => {
      do_websocket_operation::<GetReportCount>(context, id, op, data).await
    }

    // Private Message ops
    UserOperation::MarkPrivateMessageAsRead => {
      do_websocket_operation::<MarkPrivateMessageAsRead>(context, id, op, data).await
    }

    // Site ops
    UserOperation::GetModlog => do_websocket_operation::<GetModlog>(context, id, op, data).await,
    UserOperation::GetSiteConfig => {
      do_websocket_operation::<GetSiteConfig>(context, id, op, data).await
    }
    UserOperation::SaveSiteConfig => {
      do_websocket_operation::<SaveSiteConfig>(context, id, op, data).await
    }
    UserOperation::Search => do_websocket_operation::<Search>(context, id, op, data).await,
    UserOperation::TransferCommunity => {
      do_websocket_operation::<TransferCommunity>(context, id, op, data).await
    }
    UserOperation::TransferSite => {
      do_websocket_operation::<TransferSite>(context, id, op, data).await
    }

    // Community ops
    UserOperation::FollowCommunity => {
      do_websocket_operation::<FollowCommunity>(context, id, op, data).await
    }
    UserOperation::BlockCommunity => {
      do_websocket_operation::<BlockCommunity>(context, id, op, data).await
    }
    UserOperation::GetFollowedCommunities => {
      do_websocket_operation::<GetFollowedCommunities>(context, id, op, data).await
    }
    UserOperation::GetBlockedCommunities => {
      do_websocket_operation::<GetBlockedCommunities>(context, id, op, data).await
    }
    UserOperation::BanFromCommunity => {
      do_websocket_operation::<BanFromCommunity>(context, id, op, data).await
    }
    UserOperation::AddModToCommunity => {
      do_websocket_operation::<AddModToCommunity>(context, id, op, data).await
    }

    // Post ops
    UserOperation::LockPost => do_websocket_operation::<LockPost>(context, id, op, data).await,
    UserOperation::StickyPost => do_websocket_operation::<StickyPost>(context, id, op, data).await,
    UserOperation::CreatePostLike => {
      do_websocket_operation::<CreatePostLike>(context, id, op, data).await
    }
    UserOperation::SavePost => do_websocket_operation::<SavePost>(context, id, op, data).await,
    UserOperation::CreatePostReport => {
      do_websocket_operation::<CreatePostReport>(context, id, op, data).await
    }
    UserOperation::ListPostReports => {
      do_websocket_operation::<ListPostReports>(context, id, op, data).await
    }
    UserOperation::ResolvePostReport => {
      do_websocket_operation::<ResolvePostReport>(context, id, op, data).await
    }

    // Comment ops
    UserOperation::MarkCommentAsRead => {
      do_websocket_operation::<MarkCommentAsRead>(context, id, op, data).await
    }
    UserOperation::SaveComment => {
      do_websocket_operation::<SaveComment>(context, id, op, data).await
    }
    UserOperation::CreateCommentLike => {
      do_websocket_operation::<CreateCommentLike>(context, id, op, data).await
    }
    UserOperation::CreateCommentReport => {
      do_websocket_operation::<CreateCommentReport>(context, id, op, data).await
    }
    UserOperation::ListCommentReports => {
      do_websocket_operation::<ListCommentReports>(context, id, op, data).await
    }
    UserOperation::ResolveCommentReport => {
      do_websocket_operation::<ResolveCommentReport>(context, id, op, data).await
    }
  }
}

async fn do_websocket_operation<'a, 'b, Data>(
  context: LemmyContext,
  id: ConnectionId,
  op: UserOperation,
  data: &str,
) -> Result<String, LemmyError>
where
  for<'de> Data: Deserialize<'de> + 'a,
  Data: Perform,
{
  let parsed_data: Data = serde_json::from_str(data)?;
  let res = parsed_data
    .perform(&web::Data::new(context), Some(id))
    .await?;
  serialize_websocket_message(&op, &res)
}

/// Converts the captcha to a base64 encoded wav audio file
pub(crate) fn captcha_as_wav_base64(captcha: &Captcha) -> String {
  let letters = captcha.as_wav();

  let mut concat_letters: Vec<u8> = Vec::new();

  for letter in letters {
    let bytes = letter.unwrap_or_default();
    concat_letters.extend(bytes);
  }

  // Convert to base64
  base64::encode(concat_letters)
}

#[cfg(test)]
mod tests {
  use lemmy_api_common::check_validator_time;
  use lemmy_db_queries::{establish_unpooled_connection, source::local_user::LocalUser_, Crud};
  use lemmy_db_schema::source::{
    local_user::{LocalUser, LocalUserForm},
    person::{Person, PersonForm},
  };
  use lemmy_utils::claims::Claims;

  #[test]
  fn test_should_not_validate_user_token_after_password_change() {
    let conn = establish_unpooled_connection();

    let new_person = PersonForm {
      name: "Gerry9812".into(),
      ..PersonForm::default()
    };

    let inserted_person = Person::create(&conn, &new_person).unwrap();

    let local_user_form = LocalUserForm {
      person_id: inserted_person.id,
      password_encrypted: "123456".to_string(),
      ..LocalUserForm::default()
    };

    let inserted_local_user = LocalUser::create(&conn, &local_user_form).unwrap();

    let jwt = Claims::jwt(inserted_local_user.id.0).unwrap();
    let claims = Claims::decode(&jwt).unwrap().claims;
    let check = check_validator_time(&inserted_local_user.validator_time, &claims);
    assert!(check.is_ok());

    // The check should fail, since the validator time is now newer than the jwt issue time
    let updated_local_user =
      LocalUser::update_password(&conn, inserted_local_user.id, "password111").unwrap();
    let check_after = check_validator_time(&updated_local_user.validator_time, &claims);
    assert!(check_after.is_err());

    let num_deleted = Person::delete(&conn, inserted_person.id).unwrap();
    assert_eq!(1, num_deleted);
  }
}

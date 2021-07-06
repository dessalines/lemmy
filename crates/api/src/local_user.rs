use crate::{captcha_as_wav_base64, Perform};
use actix_web::web::Data;
use anyhow::Context;
use bcrypt::verify;
use captcha::{gen, Difficulty};
use chrono::Duration;
use lemmy_api_common::{
  blocking,
  collect_moderated_communities,
  community::{
    GetBlockedCommunities,
    GetBlockedCommunitiesResponse,
    GetFollowedCommunities,
    GetFollowedCommunitiesResponse,
  },
  get_local_user_view_from_jwt,
  is_admin,
  password_length_check,
  person::*,
};
use lemmy_db_queries::{
  diesel_option_overwrite,
  diesel_option_overwrite_to_url,
  from_opt_str_to_opt_enum,
  source::{
    comment::Comment_,
    local_user::LocalUser_,
    password_reset_request::PasswordResetRequest_,
    person::Person_,
    person_mention::PersonMention_,
    post::Post_,
    private_message::PrivateMessage_,
  },
  Blockable,
  Crud,
  SortType,
};
use lemmy_db_schema::{
  naive_now,
  source::{
    comment::Comment,
    local_user::{LocalUser, LocalUserForm},
    moderator::*,
    password_reset_request::*,
    person::*,
    person_block::{PersonBlock, PersonBlockForm},
    person_mention::*,
    post::Post,
    private_message::PrivateMessage,
    site::*,
  },
};
use lemmy_db_views::{
  comment_report_view::CommentReportView,
  comment_view::CommentQueryBuilder,
  local_user_view::LocalUserView,
  post_report_view::PostReportView,
};
use lemmy_db_views_actor::{
  community_block_view::CommunityBlockView,
  community_follower_view::CommunityFollowerView,
  person_block_view::PersonBlockView,
  person_mention_view::{PersonMentionQueryBuilder, PersonMentionView},
  person_view::PersonViewSafe,
};
use lemmy_utils::{
  claims::Claims,
  email::send_email,
  location_info,
  settings::structs::Settings,
  utils::{generate_random_string, is_valid_display_name, is_valid_matrix_id, naive_from_unix},
  ApiError,
  ConnectionId,
  LemmyError,
};
use lemmy_websocket::{
  messages::{CaptchaItem, SendAllMessage, SendUserRoomMessage},
  LemmyContext,
  UserOperation,
};

#[async_trait::async_trait(?Send)]
impl Perform for Login {
  type Response = LoginResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<LoginResponse, LemmyError> {
    let data: &Login = self;

    // Fetch that username / email
    let username_or_email = data.username_or_email.clone();
    let local_user_view = blocking(context.pool(), move |conn| {
      LocalUserView::find_by_email_or_name(conn, &username_or_email)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_that_username_or_email"))?;

    // Verify the password
    let valid: bool = verify(
      &data.password,
      &local_user_view.local_user.password_encrypted,
    )
    .unwrap_or(false);
    if !valid {
      return Err(ApiError::err("password_incorrect").into());
    }

    // Return the jwt
    Ok(LoginResponse {
      jwt: Claims::jwt(local_user_view.local_user.id.0)?,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetCaptcha {
  type Response = GetCaptchaResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<Self::Response, LemmyError> {
    let captcha_settings = Settings::get().captcha();

    if !captcha_settings.enabled {
      return Ok(GetCaptchaResponse { ok: None });
    }

    let captcha = match captcha_settings.difficulty.as_str() {
      "easy" => gen(Difficulty::Easy),
      "medium" => gen(Difficulty::Medium),
      "hard" => gen(Difficulty::Hard),
      _ => gen(Difficulty::Medium),
    };

    let answer = captcha.chars_as_string();

    let png = captcha.as_base64().expect("failed to generate captcha");

    let uuid = uuid::Uuid::new_v4().to_string();

    let wav = captcha_as_wav_base64(&captcha);

    let captcha_item = CaptchaItem {
      answer,
      uuid: uuid.to_owned(),
      expires: naive_now() + Duration::minutes(10), // expires in 10 minutes
    };

    // Stores the captcha item on the queue
    context.chat_server().do_send(captcha_item);

    Ok(GetCaptchaResponse {
      ok: Some(CaptchaResponse { png, wav, uuid }),
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for SaveUserSettings {
  type Response = LoginResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<LoginResponse, LemmyError> {
    let data: &SaveUserSettings = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let avatar = diesel_option_overwrite_to_url(&data.avatar)?;
    let banner = diesel_option_overwrite_to_url(&data.banner)?;
    let email = diesel_option_overwrite(&data.email);
    let bio = diesel_option_overwrite(&data.bio);
    let display_name = diesel_option_overwrite(&data.display_name);
    let matrix_user_id = diesel_option_overwrite(&data.matrix_user_id);
    let bot_account = data.bot_account;

    if let Some(Some(bio)) = &bio {
      if bio.chars().count() > 300 {
        return Err(ApiError::err("bio_length_overflow").into());
      }
    }

    if let Some(Some(display_name)) = &display_name {
      if !is_valid_display_name(display_name.trim()) {
        return Err(ApiError::err("invalid_username").into());
      }
    }

    if let Some(Some(matrix_user_id)) = &matrix_user_id {
      if !is_valid_matrix_id(matrix_user_id) {
        return Err(ApiError::err("invalid_matrix_id").into());
      }
    }

    let local_user_id = local_user_view.local_user.id;
    let person_id = local_user_view.person.id;
    let default_listing_type = data.default_listing_type;
    let default_sort_type = data.default_sort_type;
    let password_encrypted = local_user_view.local_user.password_encrypted;

    let person_form = PersonForm {
      name: local_user_view.person.name,
      avatar,
      banner,
      inbox_url: None,
      display_name,
      published: None,
      updated: Some(naive_now()),
      banned: None,
      deleted: None,
      actor_id: None,
      bio,
      local: None,
      admin: None,
      private_key: None,
      public_key: None,
      last_refreshed_at: None,
      shared_inbox_url: None,
      matrix_user_id,
      bot_account,
    };

    let person_res = blocking(context.pool(), move |conn| {
      Person::update(conn, person_id, &person_form)
    })
    .await?;
    let _updated_person: Person = match person_res {
      Ok(p) => p,
      Err(_) => {
        return Err(ApiError::err("user_already_exists").into());
      }
    };

    let local_user_form = LocalUserForm {
      person_id,
      email,
      password_encrypted,
      show_nsfw: data.show_nsfw,
      show_bot_accounts: data.show_bot_accounts,
      show_scores: data.show_scores,
      theme: data.theme.to_owned(),
      default_sort_type,
      default_listing_type,
      lang: data.lang.to_owned(),
      show_avatars: data.show_avatars,
      show_read_posts: data.show_read_posts,
      send_notifications_to_email: data.send_notifications_to_email,
    };

    let local_user_res = blocking(context.pool(), move |conn| {
      LocalUser::update(conn, local_user_id, &local_user_form)
    })
    .await?;
    let updated_local_user = match local_user_res {
      Ok(u) => u,
      Err(e) => {
        let err_type = if e.to_string()
          == "duplicate key value violates unique constraint \"local_user_email_key\""
        {
          "email_already_exists"
        } else {
          "user_already_exists"
        };

        return Err(ApiError::err(err_type).into());
      }
    };

    // Return the jwt
    Ok(LoginResponse {
      jwt: Claims::jwt(updated_local_user.id.0)?,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for ChangePassword {
  type Response = LoginResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<LoginResponse, LemmyError> {
    let data: &ChangePassword = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    password_length_check(&data.new_password)?;

    // Make sure passwords match
    if data.new_password != data.new_password_verify {
      return Err(ApiError::err("passwords_dont_match").into());
    }

    // Check the old password
    let valid: bool = verify(
      &data.old_password,
      &local_user_view.local_user.password_encrypted,
    )
    .unwrap_or(false);
    if !valid {
      return Err(ApiError::err("password_incorrect").into());
    }

    let local_user_id = local_user_view.local_user.id;
    let new_password = data.new_password.to_owned();
    let updated_local_user = blocking(context.pool(), move |conn| {
      LocalUser::update_password(conn, local_user_id, &new_password)
    })
    .await??;

    // Return the jwt
    Ok(LoginResponse {
      jwt: Claims::jwt(updated_local_user.id.0)?,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for AddAdmin {
  type Response = AddAdminResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<AddAdminResponse, LemmyError> {
    let data: &AddAdmin = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    // Make sure user is an admin
    is_admin(&local_user_view)?;

    let added = data.added;
    let added_person_id = data.person_id;
    let added_admin = match blocking(context.pool(), move |conn| {
      Person::add_admin(conn, added_person_id, added)
    })
    .await?
    {
      Ok(a) => a,
      Err(_) => {
        return Err(ApiError::err("couldnt_update_user").into());
      }
    };

    // Mod tables
    let form = ModAddForm {
      mod_person_id: local_user_view.person.id,
      other_person_id: added_admin.id,
      removed: Some(!data.added),
    };

    blocking(context.pool(), move |conn| ModAdd::create(conn, &form)).await??;

    let site_creator_id = blocking(context.pool(), move |conn| {
      Site::read(conn, 1).map(|s| s.creator_id)
    })
    .await??;

    let mut admins = blocking(context.pool(), move |conn| PersonViewSafe::admins(conn)).await??;
    let creator_index = admins
      .iter()
      .position(|r| r.person.id == site_creator_id)
      .context(location_info!())?;
    let creator_person = admins.remove(creator_index);
    admins.insert(0, creator_person);

    let res = AddAdminResponse { admins };

    context.chat_server().do_send(SendAllMessage {
      op: UserOperation::AddAdmin,
      response: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for BanPerson {
  type Response = BanPersonResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<BanPersonResponse, LemmyError> {
    let data: &BanPerson = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    // Make sure user is an admin
    is_admin(&local_user_view)?;

    let ban = data.ban;
    let banned_person_id = data.person_id;
    let ban_person = move |conn: &'_ _| Person::ban_person(conn, banned_person_id, ban);
    if blocking(context.pool(), ban_person).await?.is_err() {
      return Err(ApiError::err("couldnt_update_user").into());
    }

    // Remove their data if that's desired
    if data.remove_data.unwrap_or(false) {
      // Posts
      blocking(context.pool(), move |conn: &'_ _| {
        Post::update_removed_for_creator(conn, banned_person_id, None, true)
      })
      .await??;

      // Communities
      // Remove all communities where they're the top mod
      // TODO couldn't get group by's working in diesel,
      // for now, remove the communities manually

      // Comments
      blocking(context.pool(), move |conn: &'_ _| {
        Comment::update_removed_for_creator(conn, banned_person_id, true)
      })
      .await??;
    }

    // Mod tables
    let expires = data.expires.map(naive_from_unix);

    let form = ModBanForm {
      mod_person_id: local_user_view.person.id,
      other_person_id: data.person_id,
      reason: data.reason.to_owned(),
      banned: Some(data.ban),
      expires,
    };

    blocking(context.pool(), move |conn| ModBan::create(conn, &form)).await??;

    let person_id = data.person_id;
    let person_view = blocking(context.pool(), move |conn| {
      PersonViewSafe::read(conn, person_id)
    })
    .await??;

    let res = BanPersonResponse {
      person_view,
      banned: data.ban,
    };

    context.chat_server().do_send(SendAllMessage {
      op: UserOperation::BanPerson,
      response: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for BlockPerson {
  type Response = BlockPersonResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<BlockPersonResponse, LemmyError> {
    let data: &BlockPerson = &self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let recipient_id = data.person_id;
    let person_id = local_user_view.person.id;
    let community_block_form = PersonBlockForm {
      person_id,
      recipient_id,
    };

    if data.block {
      let block = move |conn: &'_ _| PersonBlock::block(conn, &community_block_form);
      if blocking(context.pool(), block).await?.is_err() {
        return Err(ApiError::err("person_block_already_exists").into());
      }
    } else {
      let unblock = move |conn: &'_ _| PersonBlock::unblock(conn, &community_block_form);
      if blocking(context.pool(), unblock).await?.is_err() {
        return Err(ApiError::err("person_block_already_exists").into());
      }
    }

    // TODO does any federated stuff need to be done here?

    let person_view = blocking(context.pool(), move |conn| {
      PersonViewSafe::read(conn, recipient_id)
    })
    .await??;

    let res = BlockPersonResponse {
      person_view,
      blocked: data.block,
    };

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetReplies {
  type Response = GetRepliesResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetRepliesResponse, LemmyError> {
    let data: &GetReplies = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let sort: Option<SortType> = from_opt_str_to_opt_enum(&data.sort);

    let page = data.page;
    let limit = data.limit;
    let unread_only = data.unread_only;
    let person_id = local_user_view.person.id;
    let show_bot_accounts = local_user_view.local_user.show_bot_accounts;

    let replies = blocking(context.pool(), move |conn| {
      CommentQueryBuilder::create(conn)
        .sort(sort)
        .unread_only(unread_only)
        .recipient_id(person_id)
        .show_bot_accounts(show_bot_accounts)
        .my_person_id(person_id)
        .page(page)
        .limit(limit)
        .list()
    })
    .await??;

    Ok(GetRepliesResponse { replies })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetPersonMentions {
  type Response = GetPersonMentionsResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetPersonMentionsResponse, LemmyError> {
    let data: &GetPersonMentions = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let sort: Option<SortType> = from_opt_str_to_opt_enum(&data.sort);

    let page = data.page;
    let limit = data.limit;
    let unread_only = data.unread_only;
    let person_id = local_user_view.person.id;
    let mentions = blocking(context.pool(), move |conn| {
      PersonMentionQueryBuilder::create(conn)
        .recipient_id(person_id)
        .my_person_id(person_id)
        .sort(sort)
        .unread_only(unread_only)
        .page(page)
        .limit(limit)
        .list()
    })
    .await??;

    Ok(GetPersonMentionsResponse { mentions })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for MarkPersonMentionAsRead {
  type Response = PersonMentionResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<PersonMentionResponse, LemmyError> {
    let data: &MarkPersonMentionAsRead = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let person_mention_id = data.person_mention_id;
    let read_person_mention = blocking(context.pool(), move |conn| {
      PersonMention::read(conn, person_mention_id)
    })
    .await??;

    if local_user_view.person.id != read_person_mention.recipient_id {
      return Err(ApiError::err("couldnt_update_comment").into());
    }

    let person_mention_id = read_person_mention.id;
    let read = data.read;
    let update_mention =
      move |conn: &'_ _| PersonMention::update_read(conn, person_mention_id, read);
    if blocking(context.pool(), update_mention).await?.is_err() {
      return Err(ApiError::err("couldnt_update_comment").into());
    };

    let person_mention_id = read_person_mention.id;
    let person_id = local_user_view.person.id;
    let person_mention_view = blocking(context.pool(), move |conn| {
      PersonMentionView::read(conn, person_mention_id, Some(person_id))
    })
    .await??;

    Ok(PersonMentionResponse {
      person_mention_view,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for MarkAllAsRead {
  type Response = GetRepliesResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetRepliesResponse, LemmyError> {
    let data: &MarkAllAsRead = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let person_id = local_user_view.person.id;
    let replies = blocking(context.pool(), move |conn| {
      CommentQueryBuilder::create(conn)
        .my_person_id(person_id)
        .recipient_id(person_id)
        .unread_only(true)
        .page(1)
        .limit(999)
        .list()
    })
    .await??;

    // TODO: this should probably be a bulk operation
    // Not easy to do as a bulk operation,
    // because recipient_id isn't in the comment table
    for comment_view in &replies {
      let reply_id = comment_view.comment.id;
      let mark_as_read = move |conn: &'_ _| Comment::update_read(conn, reply_id, true);
      if blocking(context.pool(), mark_as_read).await?.is_err() {
        return Err(ApiError::err("couldnt_update_comment").into());
      }
    }

    // Mark all user mentions as read
    let update_person_mentions =
      move |conn: &'_ _| PersonMention::mark_all_as_read(conn, person_id);
    if blocking(context.pool(), update_person_mentions)
      .await?
      .is_err()
    {
      return Err(ApiError::err("couldnt_update_comment").into());
    }

    // Mark all private_messages as read
    let update_pm = move |conn: &'_ _| PrivateMessage::mark_all_as_read(conn, person_id);
    if blocking(context.pool(), update_pm).await?.is_err() {
      return Err(ApiError::err("couldnt_update_private_message").into());
    }

    Ok(GetRepliesResponse { replies: vec![] })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for PasswordReset {
  type Response = PasswordResetResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<PasswordResetResponse, LemmyError> {
    let data: &PasswordReset = self;

    // Fetch that email
    let email = data.email.clone();
    let local_user_view = blocking(context.pool(), move |conn| {
      LocalUserView::find_by_email(conn, &email)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_that_username_or_email"))?;

    // Generate a random token
    let token = generate_random_string();

    // Insert the row
    let token2 = token.clone();
    let local_user_id = local_user_view.local_user.id;
    blocking(context.pool(), move |conn| {
      PasswordResetRequest::create_token(conn, local_user_id, &token2)
    })
    .await??;

    // Email the pure token to the user.
    // TODO no i18n support here.
    let email = &local_user_view.local_user.email.expect("email");
    let subject = &format!("Password reset for {}", local_user_view.person.name);
    let hostname = &Settings::get().get_protocol_and_hostname();
    let html = &format!("<h1>Password Reset Request for {}</h1><br><a href={}/password_change/{}>Click here to reset your password</a>", local_user_view.person.name, hostname, &token);
    send_email(subject, email, &local_user_view.person.name, html)
      .map_err(|e| ApiError::err(&e))?;

    Ok(PasswordResetResponse {})
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for PasswordChange {
  type Response = LoginResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<LoginResponse, LemmyError> {
    let data: &PasswordChange = self;

    // Fetch the user_id from the token
    let token = data.token.clone();
    let local_user_id = blocking(context.pool(), move |conn| {
      PasswordResetRequest::read_from_token(conn, &token).map(|p| p.local_user_id)
    })
    .await??;

    password_length_check(&data.password)?;

    // Make sure passwords match
    if data.password != data.password_verify {
      return Err(ApiError::err("passwords_dont_match").into());
    }

    // Update the user with the new password
    let password = data.password.clone();
    let updated_local_user = blocking(context.pool(), move |conn| {
      LocalUser::update_password(conn, local_user_id, &password)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_update_user"))?;

    // Return the jwt
    Ok(LoginResponse {
      jwt: Claims::jwt(updated_local_user.id.0)?,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetReportCount {
  type Response = GetReportCountResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<GetReportCountResponse, LemmyError> {
    let data: &GetReportCount = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let person_id = local_user_view.person.id;
    let community_id = data.community;
    let community_ids =
      collect_moderated_communities(person_id, community_id, context.pool()).await?;

    let res = {
      if community_ids.is_empty() {
        GetReportCountResponse {
          community: None,
          comment_reports: 0,
          post_reports: 0,
        }
      } else {
        let ids = community_ids.clone();
        let comment_reports = blocking(context.pool(), move |conn| {
          CommentReportView::get_report_count(conn, &ids)
        })
        .await??;

        let ids = community_ids.clone();
        let post_reports = blocking(context.pool(), move |conn| {
          PostReportView::get_report_count(conn, &ids)
        })
        .await??;

        GetReportCountResponse {
          community: data.community,
          comment_reports,
          post_reports,
        }
      }
    };

    context.chat_server().do_send(SendUserRoomMessage {
      op: UserOperation::GetReportCount,
      response: res.clone(),
      local_recipient_id: local_user_view.local_user.id,
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetFollowedCommunities {
  type Response = GetFollowedCommunitiesResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetFollowedCommunitiesResponse, LemmyError> {
    let data: &GetFollowedCommunities = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let person_id = local_user_view.person.id;
    let communities = blocking(context.pool(), move |conn| {
      CommunityFollowerView::for_person(conn, person_id)
    })
    .await?
    .map_err(|_| ApiError::err("system_err_login"))?;

    // Return the jwt
    Ok(GetFollowedCommunitiesResponse { communities })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetBlockedCommunities {
  type Response = GetBlockedCommunitiesResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetBlockedCommunitiesResponse, LemmyError> {
    let data: &GetBlockedCommunities = &self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let person_id = local_user_view.person.id;
    let communities = blocking(context.pool(), move |conn| {
      CommunityBlockView::for_person(conn, person_id)
    })
    .await?
    .map_err(|_| ApiError::err("system_err_login"))?;

    // Return the jwt
    Ok(GetBlockedCommunitiesResponse { communities })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for GetBlockedPersons {
  type Response = GetBlockedPersonsResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetBlockedPersonsResponse, LemmyError> {
    let data: &GetBlockedPersons = &self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let person_id = local_user_view.person.id;
    let persons = blocking(context.pool(), move |conn| {
      PersonBlockView::for_person(conn, person_id)
    })
    .await?
    .map_err(|_| ApiError::err("system_err_login"))?;

    // Return the jwt
    Ok(GetBlockedPersonsResponse { persons })
  }
}

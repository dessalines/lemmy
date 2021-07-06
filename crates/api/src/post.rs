use crate::Perform;
use actix_web::web::Data;
use lemmy_api_common::{
  blocking,
  check_community_ban,
  check_downvotes_enabled,
  check_person_block,
  get_local_user_view_from_jwt,
  is_mod_or_admin,
  mark_post_as_read,
  post::*,
};
use lemmy_apub::{ApubLikeableType, ApubObjectType};
use lemmy_db_queries::{source::post::Post_, Crud, Likeable, Saveable};
use lemmy_db_schema::source::{moderator::*, post::*};
use lemmy_db_views::post_view::PostView;
use lemmy_utils::{ApiError, ConnectionId, LemmyError};
use lemmy_websocket::{messages::SendPost, LemmyContext, UserOperation};

#[async_trait::async_trait(?Send)]
impl Perform for CreatePostLike {
  type Response = PostResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<PostResponse, LemmyError> {
    let data: &CreatePostLike = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    // Don't do a downvote if site has downvotes disabled
    check_downvotes_enabled(data.score, context.pool()).await?;

    // Check for a community ban
    let post_id = data.post_id;
    let post = blocking(context.pool(), move |conn| Post::read(conn, post_id)).await??;

    check_community_ban(local_user_view.person.id, post.community_id, context.pool()).await?;

    check_person_block(local_user_view.person.id, post.creator_id, context.pool()).await?;

    let like_form = PostLikeForm {
      post_id: data.post_id,
      person_id: local_user_view.person.id,
      score: data.score,
    };

    // Remove any likes first
    let person_id = local_user_view.person.id;
    blocking(context.pool(), move |conn| {
      PostLike::remove(conn, person_id, post_id)
    })
    .await??;

    // Only add the like if the score isnt 0
    let do_add = like_form.score != 0 && (like_form.score == 1 || like_form.score == -1);
    if do_add {
      let like_form2 = like_form.clone();
      let like = move |conn: &'_ _| PostLike::like(conn, &like_form2);
      if blocking(context.pool(), like).await?.is_err() {
        return Err(ApiError::err("couldnt_like_post").into());
      }

      if like_form.score == 1 {
        post.send_like(&local_user_view.person, context).await?;
      } else if like_form.score == -1 {
        post.send_dislike(&local_user_view.person, context).await?;
      }
    } else {
      post
        .send_undo_like(&local_user_view.person, context)
        .await?;
    }

    // Mark the post as read
    mark_post_as_read(person_id, post_id, context.pool()).await?;

    let post_id = data.post_id;
    let person_id = local_user_view.person.id;
    let post_view = blocking(context.pool(), move |conn| {
      PostView::read(conn, post_id, Some(person_id))
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_post"))?;

    let res = PostResponse { post_view };

    context.chat_server().do_send(SendPost {
      op: UserOperation::CreatePostLike,
      post: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for LockPost {
  type Response = PostResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<PostResponse, LemmyError> {
    let data: &LockPost = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let post_id = data.post_id;
    let orig_post = blocking(context.pool(), move |conn| Post::read(conn, post_id)).await??;

    check_community_ban(
      local_user_view.person.id,
      orig_post.community_id,
      context.pool(),
    )
    .await?;

    // Verify that only the mods can lock
    is_mod_or_admin(
      context.pool(),
      local_user_view.person.id,
      orig_post.community_id,
    )
    .await?;

    // Update the post
    let post_id = data.post_id;
    let locked = data.locked;
    let updated_post = blocking(context.pool(), move |conn| {
      Post::update_locked(conn, post_id, locked)
    })
    .await??;

    // Mod tables
    let form = ModLockPostForm {
      mod_person_id: local_user_view.person.id,
      post_id: data.post_id,
      locked: Some(locked),
    };
    blocking(context.pool(), move |conn| ModLockPost::create(conn, &form)).await??;

    // apub updates
    updated_post
      .send_update(&local_user_view.person, context)
      .await?;

    // Refetch the post
    let post_id = data.post_id;
    let post_view = blocking(context.pool(), move |conn| {
      PostView::read(conn, post_id, Some(local_user_view.person.id))
    })
    .await??;

    let res = PostResponse { post_view };

    context.chat_server().do_send(SendPost {
      op: UserOperation::LockPost,
      post: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for StickyPost {
  type Response = PostResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<PostResponse, LemmyError> {
    let data: &StickyPost = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let post_id = data.post_id;
    let orig_post = blocking(context.pool(), move |conn| Post::read(conn, post_id)).await??;

    check_community_ban(
      local_user_view.person.id,
      orig_post.community_id,
      context.pool(),
    )
    .await?;

    // Verify that only the mods can sticky
    is_mod_or_admin(
      context.pool(),
      local_user_view.person.id,
      orig_post.community_id,
    )
    .await?;

    // Update the post
    let post_id = data.post_id;
    let stickied = data.stickied;
    let updated_post = blocking(context.pool(), move |conn| {
      Post::update_stickied(conn, post_id, stickied)
    })
    .await??;

    // Mod tables
    let form = ModStickyPostForm {
      mod_person_id: local_user_view.person.id,
      post_id: data.post_id,
      stickied: Some(stickied),
    };
    blocking(context.pool(), move |conn| {
      ModStickyPost::create(conn, &form)
    })
    .await??;

    // Apub updates
    // TODO stickied should pry work like locked for ease of use
    updated_post
      .send_update(&local_user_view.person, context)
      .await?;

    // Refetch the post
    let post_id = data.post_id;
    let post_view = blocking(context.pool(), move |conn| {
      PostView::read(conn, post_id, Some(local_user_view.person.id))
    })
    .await??;

    let res = PostResponse { post_view };

    context.chat_server().do_send(SendPost {
      op: UserOperation::StickyPost,
      post: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for SavePost {
  type Response = PostResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<PostResponse, LemmyError> {
    let data: &SavePost = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let post_saved_form = PostSavedForm {
      post_id: data.post_id,
      person_id: local_user_view.person.id,
    };

    if data.save {
      let save = move |conn: &'_ _| PostSaved::save(conn, &post_saved_form);
      if blocking(context.pool(), save).await?.is_err() {
        return Err(ApiError::err("couldnt_save_post").into());
      }
    } else {
      let unsave = move |conn: &'_ _| PostSaved::unsave(conn, &post_saved_form);
      if blocking(context.pool(), unsave).await?.is_err() {
        return Err(ApiError::err("couldnt_save_post").into());
      }
    }

    let post_id = data.post_id;
    let person_id = local_user_view.person.id;
    let post_view = blocking(context.pool(), move |conn| {
      PostView::read(conn, post_id, Some(person_id))
    })
    .await??;

    // Mark the post as read
    mark_post_as_read(person_id, post_id, context.pool()).await?;

    Ok(PostResponse { post_view })
  }
}

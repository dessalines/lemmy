use crate::PerformCrud;
use actix_web::web::Data;
use lemmy_api_common::{blocking, get_local_user_view_from_jwt_opt, mark_post_as_read, post::*};
use lemmy_db_queries::{from_opt_str_to_opt_enum, ListingType, SortType};
use lemmy_db_views::{
  comment_view::CommentQueryBuilder,
  post_view::{PostQueryBuilder, PostView},
};
use lemmy_db_views_actor::{
  community_moderator_view::CommunityModeratorView,
  community_view::CommunityView,
};
use lemmy_utils::{ApiError, ConnectionId, LemmyError};
use lemmy_websocket::{messages::GetPostUsersOnline, LemmyContext};

#[async_trait::async_trait(?Send)]
impl PerformCrud for GetPost {
  type Response = GetPostResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetPostResponse, LemmyError> {
    let data: &GetPost = &self;
    let local_user_view = get_local_user_view_from_jwt_opt(&data.auth, context.pool()).await?;

    let show_bot_accounts = local_user_view
      .as_ref()
      .map(|t| t.local_user.show_bot_accounts);
    let person_id = local_user_view.map(|u| u.person.id);

    let id = data.id;
    let post_view = blocking(context.pool(), move |conn| {
      PostView::read(conn, id, person_id)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_post"))?;

    // Mark the post as read
    if let Some(person_id) = person_id {
      mark_post_as_read(person_id, id, context.pool()).await?;
    }

    let id = data.id;
    let comments = blocking(context.pool(), move |conn| {
      CommentQueryBuilder::create(conn)
        .my_person_id(person_id)
        .show_bot_accounts(show_bot_accounts)
        .post_id(id)
        .limit(9999)
        .list()
    })
    .await??;

    let community_id = post_view.community.id;
    let moderators = blocking(context.pool(), move |conn| {
      CommunityModeratorView::for_community(conn, community_id)
    })
    .await??;

    // Necessary for the sidebar
    let community_view = blocking(context.pool(), move |conn| {
      CommunityView::read(conn, community_id, person_id)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_community"))?;

    let online = context
      .chat_server()
      .send(GetPostUsersOnline { post_id: data.id })
      .await
      .unwrap_or(1);

    // Return the jwt
    Ok(GetPostResponse {
      post_view,
      community_view,
      comments,
      moderators,
      online,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl PerformCrud for GetPosts {
  type Response = GetPostsResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetPostsResponse, LemmyError> {
    let data: &GetPosts = &self;
    let local_user_view = get_local_user_view_from_jwt_opt(&data.auth, context.pool()).await?;

    let person_id = local_user_view.to_owned().map(|l| l.person.id);

    let show_nsfw = local_user_view.as_ref().map(|t| t.local_user.show_nsfw);
    let show_bot_accounts = local_user_view
      .as_ref()
      .map(|t| t.local_user.show_bot_accounts);
    let show_read_posts = local_user_view
      .as_ref()
      .map(|t| t.local_user.show_read_posts);

    let sort: Option<SortType> = from_opt_str_to_opt_enum(&data.sort);
    let listing_type: Option<ListingType> = from_opt_str_to_opt_enum(&data.type_);

    let page = data.page;
    let limit = data.limit;
    let community_id = data.community_id;
    let community_name = data.community_name.to_owned();
    let saved_only = data.saved_only;
    let languages = local_user_view.map(|l| l.local_user.discussion_languages);

    let posts = blocking(context.pool(), move |conn| {
      PostQueryBuilder::create(conn)
        .listing_type(listing_type)
        .sort(sort)
        .show_nsfw(show_nsfw)
        .show_bot_accounts(show_bot_accounts)
        .show_read_posts(show_read_posts)
        .community_id(community_id)
        .community_name(community_name)
        .saved_only(saved_only)
        .my_person_id(person_id)
        .page(page)
        .limit(limit)
        .languages(languages)
        .list()
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_get_posts"))?;

    Ok(GetPostsResponse { posts })
  }
}

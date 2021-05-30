use lemmy_db_schema::{CommunityId, PostId};
use lemmy_db_views::{
  comment_view::CommentView,
  post_report_view::PostReportView,
  post_view::PostView,
};
use lemmy_db_views_actor::{
  community_moderator_view::CommunityModeratorView,
  community_view::CommunityView,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize, Debug)]
pub struct CreatePost {
  pub name: String,
  pub community_id: CommunityId,
  pub url: Option<Url>,
  pub body: Option<String>,
  pub nsfw: Option<bool>,
  pub auth: String,
  pub language: String,
}

#[derive(Serialize, Clone)]
pub struct PostResponse {
  pub post_view: PostView,
}

#[derive(Deserialize)]
pub struct GetPost {
  pub id: PostId,
  pub auth: Option<String>,
}

#[derive(Serialize)]
pub struct GetPostResponse {
  pub post_view: PostView,
  pub community_view: CommunityView,
  pub comments: Vec<CommentView>,
  pub moderators: Vec<CommunityModeratorView>,
  pub online: usize,
}

#[derive(Deserialize, Debug)]
pub struct GetPosts {
  pub type_: Option<String>,
  pub sort: Option<String>,
  pub page: Option<i64>,
  pub limit: Option<i64>,
  pub community_id: Option<CommunityId>,
  pub community_name: Option<String>,
  pub saved_only: Option<bool>,
  pub auth: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct GetPostsResponse {
  pub posts: Vec<PostView>,
}

#[derive(Deserialize)]
pub struct CreatePostLike {
  pub post_id: PostId,
  pub score: i16,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct EditPost {
  pub post_id: PostId,
  pub name: Option<String>,
  pub url: Option<Url>,
  pub body: Option<String>,
  pub nsfw: Option<bool>,
  pub auth: String,
  pub language: String,
}

#[derive(Deserialize)]
pub struct DeletePost {
  pub post_id: PostId,
  pub deleted: bool,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct RemovePost {
  pub post_id: PostId,
  pub removed: bool,
  pub reason: Option<String>,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct LockPost {
  pub post_id: PostId,
  pub locked: bool,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct StickyPost {
  pub post_id: PostId,
  pub stickied: bool,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct SavePost {
  pub post_id: PostId,
  pub save: bool,
  pub auth: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreatePostReport {
  pub post_id: PostId,
  pub reason: String,
  pub auth: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CreatePostReportResponse {
  pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResolvePostReport {
  pub report_id: i32,
  pub resolved: bool,
  pub auth: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResolvePostReportResponse {
  pub report_id: i32,
  pub resolved: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListPostReports {
  pub page: Option<i64>,
  pub limit: Option<i64>,
  pub community: Option<CommunityId>,
  pub auth: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct ListPostReportsResponse {
  pub posts: Vec<PostReportView>,
}

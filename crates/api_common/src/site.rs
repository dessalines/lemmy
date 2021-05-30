use lemmy_db_schema::{CommunityId, PersonId, PrimaryLanguageTag};
use lemmy_db_views::{
  comment_view::CommentView,
  local_user_view::LocalUserSettingsView,
  post_view::PostView,
  site_view::SiteView,
};
use lemmy_db_views_actor::{community_view::CommunityView, person_view::PersonViewSafe};
use lemmy_db_views_moderator::{
  mod_add_community_view::ModAddCommunityView,
  mod_add_view::ModAddView,
  mod_ban_from_community_view::ModBanFromCommunityView,
  mod_ban_view::ModBanView,
  mod_lock_post_view::ModLockPostView,
  mod_remove_comment_view::ModRemoveCommentView,
  mod_remove_community_view::ModRemoveCommunityView,
  mod_remove_post_view::ModRemovePostView,
  mod_sticky_post_view::ModStickyPostView,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Search {
  pub q: String,
  pub community_id: Option<CommunityId>,
  pub community_name: Option<String>,
  pub creator_id: Option<PersonId>,
  pub type_: Option<String>,
  pub sort: Option<String>,
  pub listing_type: Option<String>,
  pub page: Option<i64>,
  pub limit: Option<i64>,
  pub auth: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SearchResponse {
  pub type_: String,
  pub comments: Vec<CommentView>,
  pub posts: Vec<PostView>,
  pub communities: Vec<CommunityView>,
  pub users: Vec<PersonViewSafe>,
}

#[derive(Deserialize)]
pub struct GetModlog {
  pub mod_person_id: Option<PersonId>,
  pub community_id: Option<CommunityId>,
  pub page: Option<i64>,
  pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct GetModlogResponse {
  pub removed_posts: Vec<ModRemovePostView>,
  pub locked_posts: Vec<ModLockPostView>,
  pub stickied_posts: Vec<ModStickyPostView>,
  pub removed_comments: Vec<ModRemoveCommentView>,
  pub removed_communities: Vec<ModRemoveCommunityView>,
  pub banned_from_community: Vec<ModBanFromCommunityView>,
  pub banned: Vec<ModBanView>,
  pub added_to_community: Vec<ModAddCommunityView>,
  pub added: Vec<ModAddView>,
}

#[derive(Deserialize)]
pub struct CreateSite {
  pub name: String,
  pub sidebar: Option<String>,
  pub description: Option<String>,
  pub icon: Option<String>,
  pub banner: Option<String>,
  pub enable_downvotes: Option<bool>,
  pub open_registration: Option<bool>,
  pub enable_nsfw: Option<bool>,
  pub community_creation_admin_only: Option<bool>,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct EditSite {
  pub name: Option<String>,
  pub sidebar: Option<String>,
  pub description: Option<String>,
  pub icon: Option<String>,
  pub banner: Option<String>,
  pub enable_downvotes: Option<bool>,
  pub open_registration: Option<bool>,
  pub enable_nsfw: Option<bool>,
  pub community_creation_admin_only: Option<bool>,
  pub languages: Vec<PrimaryLanguageTag>,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct GetSite {
  pub auth: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SiteResponse {
  pub site_view: SiteView,
}

#[derive(Serialize)]
pub struct GetSiteResponse {
  pub site_view: Option<SiteView>, // Because the site might not be set up yet
  pub admins: Vec<PersonViewSafe>,
  pub banned: Vec<PersonViewSafe>,
  pub online: usize,
  pub version: String,
  pub my_user: Option<LocalUserSettingsView>,
  pub federated_instances: Option<FederatedInstances>, // Federation may be disabled
}

#[derive(Deserialize)]
pub struct TransferSite {
  pub person_id: PersonId,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct GetSiteConfig {
  pub auth: String,
}

#[derive(Serialize)]
pub struct GetSiteConfigResponse {
  pub config_hjson: String,
}

#[derive(Deserialize)]
pub struct SaveSiteConfig {
  pub config_hjson: String,
  pub auth: String,
}

#[derive(Serialize)]
pub struct FederatedInstances {
  pub linked: Vec<String>,
  pub allowed: Option<Vec<String>>,
  pub blocked: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GetLanguages {}

#[derive(Serialize, Clone)]
pub struct LanguagesResponse {
  pub languages: Vec<PrimaryLanguageTag>,
}

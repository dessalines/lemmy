#[macro_use]
extern crate strum_macros;

use crate::chat_server::ChatServer;
use actix::Addr;
use background_jobs::QueueHandle;
use lemmy_db_queries::DbPool;
use lemmy_utils::LemmyError;
use reqwest::Client;
use serde::Serialize;

pub mod chat_server;
pub mod handlers;
pub mod messages;
pub mod routes;

pub struct LemmyContext {
  pub pool: DbPool,
  pub chat_server: Addr<ChatServer>,
  pub client: Client,
  pub activity_queue: QueueHandle,
}

impl LemmyContext {
  pub fn create(
    pool: DbPool,
    chat_server: Addr<ChatServer>,
    client: Client,
    activity_queue: QueueHandle,
  ) -> LemmyContext {
    LemmyContext {
      pool,
      chat_server,
      client,
      activity_queue,
    }
  }
  pub fn pool(&self) -> &DbPool {
    &self.pool
  }
  pub fn chat_server(&self) -> &Addr<ChatServer> {
    &self.chat_server
  }
  pub fn client(&self) -> &Client {
    &self.client
  }
  pub fn activity_queue(&self) -> &QueueHandle {
    &self.activity_queue
  }
}

impl Clone for LemmyContext {
  fn clone(&self) -> Self {
    LemmyContext {
      pool: self.pool.clone(),
      chat_server: self.chat_server.clone(),
      client: self.client.clone(),
      activity_queue: self.activity_queue.clone(),
    }
  }
}

#[derive(Serialize)]
struct WebsocketResponse<T> {
  op: String,
  data: T,
}

pub fn serialize_websocket_message<OP, Response>(
  op: &OP,
  data: &Response,
) -> Result<String, LemmyError>
where
  Response: Serialize,
  OP: ToString,
{
  let response = WebsocketResponse {
    op: op.to_string(),
    data,
  };
  Ok(serde_json::to_string(&response)?)
}

#[derive(EnumString, ToString, Debug, Clone)]
pub enum UserOperation {
  Login,
  GetCaptcha,
  MarkCommentAsRead,
  SaveComment,
  CreateCommentLike,
  CreateCommentReport,
  ResolveCommentReport,
  ListCommentReports,
  CreatePostLike,
  LockPost,
  StickyPost,
  SavePost,
  CreatePostReport,
  ResolvePostReport,
  ListPostReports,
  GetReportCount,
  FollowCommunity,
  GetFollowedCommunities,
  GetReplies,
  GetPersonMentions,
  MarkPersonMentionAsRead,
  GetModlog,
  BanFromCommunity,
  AddModToCommunity,
  AddAdmin,
  BanPerson,
  Search,
  MarkAllAsRead,
  SaveUserSettings,
  TransferCommunity,
  TransferSite,
  PasswordReset,
  PasswordChange,
  MarkPrivateMessageAsRead,
  UserJoin,
  GetSiteConfig,
  SaveSiteConfig,
  PostJoin,
  CommunityJoin,
  ModJoin,
  ChangePassword,
  BlockCommunity,
  BlockPerson,
  GetBlockedCommunities,
  GetBlockedPersons,
}

#[derive(EnumString, ToString, Debug, Clone)]
pub enum UserOperationCrud {
  // Site
  CreateSite,
  GetSite,
  EditSite,
  // Community
  CreateCommunity,
  ListCommunities,
  GetCommunity,
  EditCommunity,
  DeleteCommunity,
  RemoveCommunity,
  // Post
  CreatePost,
  GetPost,
  GetPosts,
  EditPost,
  DeletePost,
  RemovePost,
  // Comment
  CreateComment,
  GetComments,
  EditComment,
  DeleteComment,
  RemoveComment,
  // User
  Register,
  GetPersonDetails,
  DeleteAccount,
  // Private Message
  CreatePrivateMessage,
  GetPrivateMessages,
  EditPrivateMessage,
  DeletePrivateMessage,
}

pub trait OperationType {}

impl OperationType for UserOperationCrud {}

impl OperationType for UserOperation {}

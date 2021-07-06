use diesel::{result::Error, *};
use lemmy_db_queries::{
  aggregates::comment_aggregates::CommentAggregates,
  functions::hot_rank,
  fuzzy_search,
  limit_and_offset,
  ListingType,
  MaybeOptional,
  SortType,
  ToSafe,
  ViewToVec,
};
use lemmy_db_schema::{
  schema::{
    comment,
    comment_aggregates,
    comment_alias_1,
    comment_like,
    comment_saved,
    community,
    community_block,
    community_follower,
    community_person_ban,
    person,
    person_alias_1,
    person_block,
    post,
  },
  source::{
    comment::{Comment, CommentAlias1, CommentSaved},
    community::{Community, CommunityFollower, CommunityPersonBan, CommunitySafe},
    person::{Person, PersonAlias1, PersonSafe, PersonSafeAlias1},
    person_block::PersonBlock,
    post::Post,
  },
  CommentId,
  CommunityId,
  PersonId,
  PostId,
};
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct CommentView {
  pub comment: Comment,
  pub creator: PersonSafe,
  pub recipient: Option<PersonSafeAlias1>, // Left joins to comment and person
  pub post: Post,
  pub community: CommunitySafe,
  pub counts: CommentAggregates,
  pub creator_banned_from_community: bool, // Left Join to CommunityPersonBan
  pub subscribed: bool,                    // Left join to CommunityFollower
  pub saved: bool,                         // Left join to CommentSaved
  pub creator_blocked: bool,               // Left join to PersonBlock
  pub my_vote: Option<i16>,                // Left join to CommentLike
}

type CommentViewTuple = (
  Comment,
  PersonSafe,
  Option<CommentAlias1>,
  Option<PersonSafeAlias1>,
  Post,
  CommunitySafe,
  CommentAggregates,
  Option<CommunityPersonBan>,
  Option<CommunityFollower>,
  Option<CommentSaved>,
  Option<PersonBlock>,
  Option<i16>,
);

impl CommentView {
  pub fn read(
    conn: &PgConnection,
    comment_id: CommentId,
    my_person_id: Option<PersonId>,
  ) -> Result<Self, Error> {
    // The left join below will return None in this case
    let person_id_join = my_person_id.unwrap_or(PersonId(-1));

    let (
      comment,
      creator,
      _parent_comment,
      recipient,
      post,
      community,
      counts,
      creator_banned_from_community,
      subscribed,
      saved,
      creator_blocked,
      comment_like,
    ) = comment::table
      .find(comment_id)
      .inner_join(person::table)
      // recipient here
      .left_join(comment_alias_1::table.on(comment_alias_1::id.nullable().eq(comment::parent_id)))
      .left_join(person_alias_1::table.on(person_alias_1::id.eq(comment_alias_1::creator_id)))
      .inner_join(post::table)
      .inner_join(community::table.on(post::community_id.eq(community::id)))
      .inner_join(comment_aggregates::table)
      .left_join(
        community_person_ban::table.on(
          community::id
            .eq(community_person_ban::community_id)
            .and(community_person_ban::person_id.eq(comment::creator_id)),
        ),
      )
      .left_join(
        community_follower::table.on(
          post::community_id
            .eq(community_follower::community_id)
            .and(community_follower::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        comment_saved::table.on(
          comment::id
            .eq(comment_saved::comment_id)
            .and(comment_saved::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        person_block::table.on(
          comment::creator_id
            .eq(person_block::recipient_id)
            .and(person_block::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        comment_like::table.on(
          comment::id
            .eq(comment_like::comment_id)
            .and(comment_like::person_id.eq(person_id_join)),
        ),
      )
      .select((
        comment::all_columns,
        Person::safe_columns_tuple(),
        comment_alias_1::all_columns.nullable(),
        PersonAlias1::safe_columns_tuple().nullable(),
        post::all_columns,
        Community::safe_columns_tuple(),
        comment_aggregates::all_columns,
        community_person_ban::all_columns.nullable(),
        community_follower::all_columns.nullable(),
        comment_saved::all_columns.nullable(),
        person_block::all_columns.nullable(),
        comment_like::score.nullable(),
      ))
      .first::<CommentViewTuple>(conn)?;

    // If a person is given, then my_vote, if None, should be 0, not null
    // Necessary to differentiate between other person's votes
    let my_vote = if my_person_id.is_some() && comment_like.is_none() {
      Some(0)
    } else {
      comment_like
    };

    Ok(CommentView {
      comment,
      recipient,
      post,
      creator,
      community,
      counts,
      creator_banned_from_community: creator_banned_from_community.is_some(),
      subscribed: subscribed.is_some(),
      saved: saved.is_some(),
      creator_blocked: creator_blocked.is_some(),
      my_vote,
    })
  }

  /// Gets the recipient person id.
  /// If there is no parent comment, its the post creator
  pub fn get_recipient_id(&self) -> PersonId {
    match &self.recipient {
      Some(parent_commenter) => parent_commenter.id,
      None => self.post.creator_id,
    }
  }
}

pub struct CommentQueryBuilder<'a> {
  conn: &'a PgConnection,
  listing_type: Option<ListingType>,
  sort: Option<SortType>,
  community_id: Option<CommunityId>,
  community_name: Option<String>,
  post_id: Option<PostId>,
  creator_id: Option<PersonId>,
  recipient_id: Option<PersonId>,
  my_person_id: Option<PersonId>,
  search_term: Option<String>,
  saved_only: Option<bool>,
  unread_only: Option<bool>,
  show_bot_accounts: Option<bool>,
  page: Option<i64>,
  limit: Option<i64>,
}

impl<'a> CommentQueryBuilder<'a> {
  pub fn create(conn: &'a PgConnection) -> Self {
    CommentQueryBuilder {
      conn,
      listing_type: None,
      sort: None,
      community_id: None,
      community_name: None,
      post_id: None,
      creator_id: None,
      recipient_id: None,
      my_person_id: None,
      search_term: None,
      saved_only: None,
      unread_only: None,
      show_bot_accounts: None,
      page: None,
      limit: None,
    }
  }

  pub fn listing_type<T: MaybeOptional<ListingType>>(mut self, listing_type: T) -> Self {
    self.listing_type = listing_type.get_optional();
    self
  }

  pub fn sort<T: MaybeOptional<SortType>>(mut self, sort: T) -> Self {
    self.sort = sort.get_optional();
    self
  }

  pub fn post_id<T: MaybeOptional<PostId>>(mut self, post_id: T) -> Self {
    self.post_id = post_id.get_optional();
    self
  }

  pub fn creator_id<T: MaybeOptional<PersonId>>(mut self, creator_id: T) -> Self {
    self.creator_id = creator_id.get_optional();
    self
  }

  pub fn recipient_id<T: MaybeOptional<PersonId>>(mut self, recipient_id: T) -> Self {
    self.recipient_id = recipient_id.get_optional();
    self
  }

  pub fn community_id<T: MaybeOptional<CommunityId>>(mut self, community_id: T) -> Self {
    self.community_id = community_id.get_optional();
    self
  }

  pub fn my_person_id<T: MaybeOptional<PersonId>>(mut self, my_person_id: T) -> Self {
    self.my_person_id = my_person_id.get_optional();
    self
  }

  pub fn community_name<T: MaybeOptional<String>>(mut self, community_name: T) -> Self {
    self.community_name = community_name.get_optional();
    self
  }

  pub fn search_term<T: MaybeOptional<String>>(mut self, search_term: T) -> Self {
    self.search_term = search_term.get_optional();
    self
  }

  pub fn saved_only<T: MaybeOptional<bool>>(mut self, saved_only: T) -> Self {
    self.saved_only = saved_only.get_optional();
    self
  }

  pub fn unread_only<T: MaybeOptional<bool>>(mut self, unread_only: T) -> Self {
    self.unread_only = unread_only.get_optional();
    self
  }

  pub fn show_bot_accounts<T: MaybeOptional<bool>>(mut self, show_bot_accounts: T) -> Self {
    self.show_bot_accounts = show_bot_accounts.get_optional();
    self
  }

  pub fn page<T: MaybeOptional<i64>>(mut self, page: T) -> Self {
    self.page = page.get_optional();
    self
  }

  pub fn limit<T: MaybeOptional<i64>>(mut self, limit: T) -> Self {
    self.limit = limit.get_optional();
    self
  }

  pub fn list(self) -> Result<Vec<CommentView>, Error> {
    use diesel::dsl::*;

    // The left join below will return None in this case
    let person_id_join = self.my_person_id.unwrap_or(PersonId(-1));

    let mut query = comment::table
      .inner_join(person::table)
      // recipient here
      .left_join(comment_alias_1::table.on(comment_alias_1::id.nullable().eq(comment::parent_id)))
      .left_join(person_alias_1::table.on(person_alias_1::id.eq(comment_alias_1::creator_id)))
      .inner_join(post::table)
      .inner_join(community::table.on(post::community_id.eq(community::id)))
      .inner_join(comment_aggregates::table)
      .left_join(
        community_person_ban::table.on(
          community::id
            .eq(community_person_ban::community_id)
            .and(community_person_ban::person_id.eq(comment::creator_id)),
        ),
      )
      .left_join(
        community_follower::table.on(
          post::community_id
            .eq(community_follower::community_id)
            .and(community_follower::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        comment_saved::table.on(
          comment::id
            .eq(comment_saved::comment_id)
            .and(comment_saved::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        person_block::table.on(
          comment::creator_id
            .eq(person_block::person_id)
            .and(person_block::recipient_id.eq(person_id_join)),
        ),
      )
      .left_join(
        community_block::table.on(
          community::id
            .eq(community_block::community_id)
            .and(community_block::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        comment_like::table.on(
          comment::id
            .eq(comment_like::comment_id)
            .and(comment_like::person_id.eq(person_id_join)),
        ),
      )
      .select((
        comment::all_columns,
        Person::safe_columns_tuple(),
        comment_alias_1::all_columns.nullable(),
        PersonAlias1::safe_columns_tuple().nullable(),
        post::all_columns,
        Community::safe_columns_tuple(),
        comment_aggregates::all_columns,
        community_person_ban::all_columns.nullable(),
        community_follower::all_columns.nullable(),
        comment_saved::all_columns.nullable(),
        person_block::all_columns.nullable(),
        comment_like::score.nullable(),
      ))
      .into_boxed();

    // The replies
    if let Some(recipient_id) = self.recipient_id {
      query = query
        // TODO needs lots of testing
        .filter(person_alias_1::id.eq(recipient_id)) // Gets the comment replies
        .or_filter(
          comment::parent_id
            .is_null()
            .and(post::creator_id.eq(recipient_id)),
        ) // Gets the top level replies
        .filter(comment::deleted.eq(false))
        .filter(comment::removed.eq(false));
    }

    if self.unread_only.unwrap_or(false) {
      query = query.filter(comment::read.eq(false));
    }

    if let Some(creator_id) = self.creator_id {
      query = query.filter(comment::creator_id.eq(creator_id));
    };

    if let Some(community_id) = self.community_id {
      query = query.filter(post::community_id.eq(community_id));
    }

    if let Some(community_name) = self.community_name {
      query = query
        .filter(community::name.eq(community_name))
        .filter(comment::local.eq(true));
    }

    if let Some(post_id) = self.post_id {
      query = query.filter(comment::post_id.eq(post_id));
    };

    if let Some(search_term) = self.search_term {
      query = query.filter(comment::content.ilike(fuzzy_search(&search_term)));
    };

    if let Some(listing_type) = self.listing_type {
      query = match listing_type {
        ListingType::Subscribed => query.filter(community_follower::person_id.is_not_null()), // TODO could be this: and(community_follower::person_id.eq(person_id_join)),
        ListingType::Local => query.filter(community::local.eq(true)),
        _ => query,
      };
    }

    if self.saved_only.unwrap_or(false) {
      query = query.filter(comment_saved::id.is_not_null());
    }

    if !self.show_bot_accounts.unwrap_or(true) {
      query = query.filter(person::bot_account.eq(false));
    };

    query = match self.sort.unwrap_or(SortType::New) {
      SortType::Hot | SortType::Active => query
        .order_by(hot_rank(comment_aggregates::score, comment_aggregates::published).desc())
        .then_order_by(comment_aggregates::published.desc()),
      SortType::New | SortType::MostComments | SortType::NewComments => {
        query.order_by(comment::published.desc())
      }
      SortType::TopAll => query.order_by(comment_aggregates::score.desc()),
      SortType::TopYear => query
        .filter(comment::published.gt(now - 1.years()))
        .order_by(comment_aggregates::score.desc()),
      SortType::TopMonth => query
        .filter(comment::published.gt(now - 1.months()))
        .order_by(comment_aggregates::score.desc()),
      SortType::TopWeek => query
        .filter(comment::published.gt(now - 1.weeks()))
        .order_by(comment_aggregates::score.desc()),
      SortType::TopDay => query
        .filter(comment::published.gt(now - 1.days()))
        .order_by(comment_aggregates::score.desc()),
    };

    // Don't show blocked communities or persons
    if self.my_person_id.is_some() {
      query = query.filter(community_block::person_id.is_null());
      query = query.filter(person_block::person_id.is_null());
    }

    let (limit, offset) = limit_and_offset(self.page, self.limit);

    // Note: deleted and removed comments are done on the front side
    let res = query
      .limit(limit)
      .offset(offset)
      .load::<CommentViewTuple>(self.conn)?;

    Ok(CommentView::from_tuple_to_vec(res))
  }
}

impl ViewToVec for CommentView {
  type DbTuple = CommentViewTuple;
  fn from_tuple_to_vec(items: Vec<Self::DbTuple>) -> Vec<Self> {
    items
      .iter()
      .map(|a| Self {
        comment: a.0.to_owned(),
        creator: a.1.to_owned(),
        recipient: a.3.to_owned(),
        post: a.4.to_owned(),
        community: a.5.to_owned(),
        counts: a.6.to_owned(),
        creator_banned_from_community: a.7.is_some(),
        subscribed: a.8.is_some(),
        saved: a.9.is_some(),
        creator_blocked: a.10.is_some(),
        my_vote: a.11,
      })
      .collect::<Vec<Self>>()
  }
}

#[cfg(test)]
mod tests {
  use crate::comment_view::*;
  use lemmy_db_queries::{
    aggregates::comment_aggregates::CommentAggregates,
    establish_unpooled_connection,
    Crud,
    Likeable,
  };
  use lemmy_db_schema::source::{comment::*, community::*, person::*, post::*};
  use serial_test::serial;

  #[test]
  #[serial]
  fn test_crud() {
    let conn = establish_unpooled_connection();

    let new_person = PersonForm {
      name: "timmy".into(),
      ..PersonForm::default()
    };

    let inserted_person = Person::create(&conn, &new_person).unwrap();

    let new_community = CommunityForm {
      name: "test community 5".to_string(),
      title: "nada".to_owned(),
      ..CommunityForm::default()
    };

    let inserted_community = Community::create(&conn, &new_community).unwrap();

    let new_post = PostForm {
      name: "A test post 2".into(),
      creator_id: inserted_person.id,
      community_id: inserted_community.id,
      ..PostForm::default()
    };

    let inserted_post = Post::create(&conn, &new_post).unwrap();

    let comment_form = CommentForm {
      content: "A test comment 32".into(),
      creator_id: inserted_person.id,
      post_id: inserted_post.id,
      ..CommentForm::default()
    };

    let inserted_comment = Comment::create(&conn, &comment_form).unwrap();

    let comment_like_form = CommentLikeForm {
      comment_id: inserted_comment.id,
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      score: 1,
    };

    let _inserted_comment_like = CommentLike::like(&conn, &comment_like_form).unwrap();

    let agg = CommentAggregates::read(&conn, inserted_comment.id).unwrap();

    let expected_comment_view_no_person = CommentView {
      creator_banned_from_community: false,
      my_vote: None,
      subscribed: false,
      saved: false,
      creator_blocked: false,
      comment: Comment {
        id: inserted_comment.id,
        content: "A test comment 32".into(),
        creator_id: inserted_person.id,
        post_id: inserted_post.id,
        parent_id: None,
        removed: false,
        deleted: false,
        read: false,
        published: inserted_comment.published,
        ap_id: inserted_comment.ap_id,
        updated: None,
        local: true,
      },
      creator: PersonSafe {
        id: inserted_person.id,
        name: "timmy".into(),
        display_name: None,
        published: inserted_person.published,
        avatar: None,
        actor_id: inserted_person.actor_id.to_owned(),
        local: true,
        banned: false,
        deleted: false,
        admin: false,
        bot_account: false,
        bio: None,
        banner: None,
        updated: None,
        inbox_url: inserted_person.inbox_url.to_owned(),
        shared_inbox_url: None,
        matrix_user_id: None,
      },
      recipient: None,
      post: Post {
        id: inserted_post.id,
        name: inserted_post.name.to_owned(),
        creator_id: inserted_person.id,
        url: None,
        body: None,
        published: inserted_post.published,
        updated: None,
        community_id: inserted_community.id,
        removed: false,
        deleted: false,
        locked: false,
        stickied: false,
        nsfw: false,
        embed_title: None,
        embed_description: None,
        embed_html: None,
        thumbnail_url: None,
        ap_id: inserted_post.ap_id.to_owned(),
        local: true,
      },
      community: CommunitySafe {
        id: inserted_community.id,
        name: "test community 5".to_string(),
        icon: None,
        removed: false,
        deleted: false,
        nsfw: false,
        actor_id: inserted_community.actor_id.to_owned(),
        local: true,
        title: "nada".to_owned(),
        description: None,
        updated: None,
        banner: None,
        published: inserted_community.published,
      },
      counts: CommentAggregates {
        id: agg.id,
        comment_id: inserted_comment.id,
        score: 1,
        upvotes: 1,
        downvotes: 0,
        published: agg.published,
      },
    };

    let mut expected_comment_view_with_person = expected_comment_view_no_person.to_owned();
    expected_comment_view_with_person.my_vote = Some(1);

    let read_comment_views_no_person = CommentQueryBuilder::create(&conn)
      .post_id(inserted_post.id)
      .list()
      .unwrap();

    let read_comment_views_with_person = CommentQueryBuilder::create(&conn)
      .post_id(inserted_post.id)
      .my_person_id(inserted_person.id)
      .list()
      .unwrap();

    let like_removed = CommentLike::remove(&conn, inserted_person.id, inserted_comment.id).unwrap();
    let num_deleted = Comment::delete(&conn, inserted_comment.id).unwrap();
    Post::delete(&conn, inserted_post.id).unwrap();
    Community::delete(&conn, inserted_community.id).unwrap();
    Person::delete(&conn, inserted_person.id).unwrap();

    assert_eq!(
      expected_comment_view_no_person,
      read_comment_views_no_person[0]
    );
    assert_eq!(
      expected_comment_view_with_person,
      read_comment_views_with_person[0]
    );
    assert_eq!(1, num_deleted);
    assert_eq!(1, like_removed);
  }
}

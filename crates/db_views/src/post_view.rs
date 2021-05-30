use diesel::{pg::Pg, result::Error, *};
use lemmy_db_queries::{
  aggregates::post_aggregates::PostAggregates,
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
    community,
    community_follower,
    community_person_ban,
    person,
    post,
    post_aggregates,
    post_like,
    post_read,
    post_saved,
  },
  source::{
    community::{Community, CommunityFollower, CommunityPersonBan, CommunitySafe},
    person::{Person, PersonSafe},
    post::{Post, PostRead, PostSaved},
  },
  CommunityId,
  PersonId,
  PostId,
  PrimaryLanguageTag,
};
use log::debug;
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct PostView {
  pub post: Post,
  pub creator: PersonSafe,
  pub community: CommunitySafe,
  pub creator_banned_from_community: bool, // Left Join to CommunityPersonBan
  pub counts: PostAggregates,
  pub subscribed: bool,     // Left join to CommunityFollower
  pub saved: bool,          // Left join to PostSaved
  pub read: bool,           // Left join to PostRead
  pub my_vote: Option<i16>, // Left join to PostLike
}

type PostViewTuple = (
  Post,
  PersonSafe,
  CommunitySafe,
  Option<CommunityPersonBan>,
  PostAggregates,
  Option<CommunityFollower>,
  Option<PostSaved>,
  Option<PostRead>,
  Option<i16>,
);

impl PostView {
  pub fn read(
    conn: &PgConnection,
    post_id: PostId,
    my_person_id: Option<PersonId>,
  ) -> Result<Self, Error> {
    // The left join below will return None in this case
    let person_id_join = my_person_id.unwrap_or(PersonId(-1));

    let (
      post,
      creator,
      community,
      creator_banned_from_community,
      counts,
      follower,
      saved,
      read,
      post_like,
    ) = post::table
      .find(post_id)
      .inner_join(person::table)
      .inner_join(community::table)
      .left_join(
        community_person_ban::table.on(
          post::community_id
            .eq(community_person_ban::community_id)
            .and(community_person_ban::person_id.eq(post::creator_id)),
        ),
      )
      .inner_join(post_aggregates::table)
      .left_join(
        community_follower::table.on(
          post::community_id
            .eq(community_follower::community_id)
            .and(community_follower::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        post_saved::table.on(
          post::id
            .eq(post_saved::post_id)
            .and(post_saved::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        post_read::table.on(
          post::id
            .eq(post_read::post_id)
            .and(post_read::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        post_like::table.on(
          post::id
            .eq(post_like::post_id)
            .and(post_like::person_id.eq(person_id_join)),
        ),
      )
      .select((
        post::all_columns,
        Person::safe_columns_tuple(),
        Community::safe_columns_tuple(),
        community_person_ban::all_columns.nullable(),
        post_aggregates::all_columns,
        community_follower::all_columns.nullable(),
        post_saved::all_columns.nullable(),
        post_read::all_columns.nullable(),
        post_like::score.nullable(),
      ))
      .first::<PostViewTuple>(conn)?;

    // If a person is given, then my_vote, if None, should be 0, not null
    // Necessary to differentiate between other person's votes
    let my_vote = if my_person_id.is_some() && post_like.is_none() {
      Some(0)
    } else {
      post_like
    };

    Ok(PostView {
      post,
      creator,
      community,
      creator_banned_from_community: creator_banned_from_community.is_some(),
      counts,
      subscribed: follower.is_some(),
      saved: saved.is_some(),
      read: read.is_some(),
      my_vote,
    })
  }
}

pub struct PostQueryBuilder<'a> {
  conn: &'a PgConnection,
  listing_type: Option<ListingType>,
  sort: Option<SortType>,
  creator_id: Option<PersonId>,
  community_id: Option<CommunityId>,
  community_name: Option<String>,
  my_person_id: Option<PersonId>,
  search_term: Option<String>,
  url_search: Option<String>,
  show_nsfw: Option<bool>,
  show_bot_accounts: Option<bool>,
  show_read_posts: Option<bool>,
  saved_only: Option<bool>,
  page: Option<i64>,
  limit: Option<i64>,
  languages: Option<Vec<PrimaryLanguageTag>>,
}

impl<'a> PostQueryBuilder<'a> {
  pub fn create(conn: &'a PgConnection) -> Self {
    PostQueryBuilder {
      conn,
      listing_type: None,
      sort: None,
      creator_id: None,
      community_id: None,
      community_name: None,
      my_person_id: None,
      search_term: None,
      url_search: None,
      show_nsfw: None,
      show_bot_accounts: None,
      show_read_posts: None,
      saved_only: None,
      page: None,
      limit: None,
      languages: None,
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

  pub fn creator_id<T: MaybeOptional<PersonId>>(mut self, creator_id: T) -> Self {
    self.creator_id = creator_id.get_optional();
    self
  }

  pub fn search_term<T: MaybeOptional<String>>(mut self, search_term: T) -> Self {
    self.search_term = search_term.get_optional();
    self
  }

  pub fn url_search<T: MaybeOptional<String>>(mut self, url_search: T) -> Self {
    self.url_search = url_search.get_optional();
    self
  }

  pub fn show_nsfw<T: MaybeOptional<bool>>(mut self, show_nsfw: T) -> Self {
    self.show_nsfw = show_nsfw.get_optional();
    self
  }

  pub fn show_bot_accounts<T: MaybeOptional<bool>>(mut self, show_bot_accounts: T) -> Self {
    self.show_bot_accounts = show_bot_accounts.get_optional();
    self
  }

  pub fn show_read_posts<T: MaybeOptional<bool>>(mut self, show_read_posts: T) -> Self {
    self.show_read_posts = show_read_posts.get_optional();
    self
  }

  pub fn saved_only<T: MaybeOptional<bool>>(mut self, saved_only: T) -> Self {
    self.saved_only = saved_only.get_optional();
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

  pub fn languages<T: MaybeOptional<Vec<PrimaryLanguageTag>>>(mut self, languages: T) -> Self {
    self.languages = languages.get_optional();
    self
  }

  pub fn list(self) -> Result<Vec<PostView>, Error> {
    use diesel::dsl::*;

    // The left join below will return None in this case
    let person_id_join = self.my_person_id.unwrap_or(PersonId(-1));

    let mut query = post::table
      .inner_join(person::table)
      .inner_join(community::table)
      .left_join(
        community_person_ban::table.on(
          post::community_id
            .eq(community_person_ban::community_id)
            .and(community_person_ban::person_id.eq(post::creator_id)),
        ),
      )
      .inner_join(post_aggregates::table)
      .left_join(
        community_follower::table.on(
          post::community_id
            .eq(community_follower::community_id)
            .and(community_follower::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        post_saved::table.on(
          post::id
            .eq(post_saved::post_id)
            .and(post_saved::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        post_read::table.on(
          post::id
            .eq(post_read::post_id)
            .and(post_read::person_id.eq(person_id_join)),
        ),
      )
      .left_join(
        post_like::table.on(
          post::id
            .eq(post_like::post_id)
            .and(post_like::person_id.eq(person_id_join)),
        ),
      )
      .select((
        post::all_columns,
        Person::safe_columns_tuple(),
        Community::safe_columns_tuple(),
        community_person_ban::all_columns.nullable(),
        post_aggregates::all_columns,
        community_follower::all_columns.nullable(),
        post_saved::all_columns.nullable(),
        post_read::all_columns.nullable(),
        post_like::score.nullable(),
      ))
      .into_boxed();

    if let Some(listing_type) = self.listing_type {
      query = match listing_type {
        ListingType::Subscribed => query.filter(community_follower::person_id.is_not_null()),
        ListingType::Local => query.filter(community::local.eq(true)),
        _ => query,
      };
    }

    if let Some(community_id) = self.community_id {
      query = query
        .filter(post::community_id.eq(community_id))
        .then_order_by(post_aggregates::stickied.desc());
    }

    if let Some(community_name) = self.community_name {
      query = query
        .filter(community::name.eq(community_name))
        .filter(community::local.eq(true))
        .then_order_by(post_aggregates::stickied.desc());
    }

    if let Some(url_search) = self.url_search {
      query = query.filter(post::url.eq(url_search));
    }

    if let Some(search_term) = self.search_term {
      let searcher = fuzzy_search(&search_term);
      query = query.filter(
        post::name
          .ilike(searcher.to_owned())
          .or(post::body.ilike(searcher)),
      );
    }

    // If its for a specific person, show the removed / deleted
    if let Some(creator_id) = self.creator_id {
      query = query.filter(post::creator_id.eq(creator_id));
    }

    if !self.show_nsfw.unwrap_or(false) {
      query = query
        .filter(post::nsfw.eq(false))
        .filter(community::nsfw.eq(false));
    };

    if !self.show_bot_accounts.unwrap_or(true) {
      query = query.filter(person::bot_account.eq(false));
    };

    if !self.show_read_posts.unwrap_or(true) {
      query = query.filter(post_read::id.is_null());
    };

    if self.saved_only.unwrap_or(false) {
      query = query.filter(post_saved::id.is_not_null());
    };

    if let Some(languages) = self.languages {
      query = query.filter(post::language.eq_any(languages.into_iter()));
    };

    query = match self.sort.unwrap_or(SortType::Hot) {
      SortType::Active => query
        .then_order_by(
          hot_rank(
            post_aggregates::score,
            post_aggregates::newest_comment_time_necro,
          )
          .desc(),
        )
        .then_order_by(post_aggregates::newest_comment_time_necro.desc()),
      SortType::Hot => query
        .then_order_by(hot_rank(post_aggregates::score, post_aggregates::published).desc())
        .then_order_by(post_aggregates::published.desc()),
      SortType::New => query.then_order_by(post_aggregates::published.desc()),
      SortType::MostComments => query.then_order_by(post_aggregates::comments.desc()),
      SortType::NewComments => query.then_order_by(post_aggregates::newest_comment_time.desc()),
      SortType::TopAll => query.then_order_by(post_aggregates::score.desc()),
      SortType::TopYear => query
        .filter(post::published.gt(now - 1.years()))
        .then_order_by(post_aggregates::score.desc()),
      SortType::TopMonth => query
        .filter(post::published.gt(now - 1.months()))
        .then_order_by(post_aggregates::score.desc()),
      SortType::TopWeek => query
        .filter(post::published.gt(now - 1.weeks()))
        .then_order_by(post_aggregates::score.desc()),
      SortType::TopDay => query
        .filter(post::published.gt(now - 1.days()))
        .then_order_by(post_aggregates::score.desc()),
    };

    let (limit, offset) = limit_and_offset(self.page, self.limit);

    query = query
      .limit(limit)
      .offset(offset)
      .filter(post::removed.eq(false))
      .filter(post::deleted.eq(false))
      .filter(community::removed.eq(false))
      .filter(community::deleted.eq(false));

    debug!("Post View Query: {:?}", debug_query::<Pg, _>(&query));

    let res = query.load::<PostViewTuple>(self.conn)?;

    Ok(PostView::from_tuple_to_vec(res))
  }
}

impl ViewToVec for PostView {
  type DbTuple = PostViewTuple;
  fn from_tuple_to_vec(items: Vec<Self::DbTuple>) -> Vec<Self> {
    items
      .iter()
      .map(|a| Self {
        post: a.0.to_owned(),
        creator: a.1.to_owned(),
        community: a.2.to_owned(),
        creator_banned_from_community: a.3.is_some(),
        counts: a.4.to_owned(),
        subscribed: a.5.is_some(),
        saved: a.6.is_some(),
        read: a.7.is_some(),
        my_vote: a.8,
      })
      .collect::<Vec<Self>>()
  }
}

#[cfg(test)]
mod tests {
  use crate::post_view::{PostQueryBuilder, PostView};
  use lemmy_db_queries::{
    aggregates::post_aggregates::PostAggregates,
    establish_unpooled_connection,
    Crud,
    Likeable,
    ListingType,
    SortType,
  };
  use lemmy_db_schema::{
    source::{community::*, person::*, post::*},
    PrimaryLanguageTag,
  };
  use serial_test::serial;

  #[test]
  #[serial]
  fn test_crud() {
    let conn = establish_unpooled_connection();

    let person_name = "tegan".to_string();
    let community_name = "test_community_3".to_string();
    let post_name = "test post 3".to_string();
    let bot_post_name = "test bot post".to_string();

    let new_person = PersonForm {
      name: person_name.to_owned(),
      ..PersonForm::default()
    };

    let inserted_person = Person::create(&conn, &new_person).unwrap();

    let new_bot = PersonForm {
      name: person_name.to_owned(),
      bot_account: Some(true),
      ..PersonForm::default()
    };

    let inserted_bot = Person::create(&conn, &new_bot).unwrap();

    let new_community = CommunityForm {
      name: community_name.to_owned(),
      title: "nada".to_owned(),
      ..CommunityForm::default()
    };

    let inserted_community = Community::create(&conn, &new_community).unwrap();

    let new_post = PostForm {
      name: post_name.to_owned(),
      creator_id: inserted_person.id,
      community_id: inserted_community.id,
      language: Some(PrimaryLanguageTag("en".to_string())),
      ..PostForm::default()
    };

    let inserted_post = Post::create(&conn, &new_post).unwrap();

    let new_bot_post = PostForm {
      name: bot_post_name,
      creator_id: inserted_bot.id,
      community_id: inserted_community.id,
      language: Some(PrimaryLanguageTag("en".to_string())),
      ..PostForm::default()
    };

    let _inserted_bot_post = Post::create(&conn, &new_bot_post).unwrap();

    let post_like_form = PostLikeForm {
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      score: 1,
    };

    let inserted_post_like = PostLike::like(&conn, &post_like_form).unwrap();

    let expected_post_like = PostLike {
      id: inserted_post_like.id,
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      published: inserted_post_like.published,
      score: 1,
    };

    let read_post_listings_with_person = PostQueryBuilder::create(&conn)
      .listing_type(ListingType::Community)
      .sort(SortType::New)
      .show_bot_accounts(false)
      .community_id(inserted_community.id)
      .my_person_id(inserted_person.id)
      .list()
      .unwrap();

    let read_post_listings_no_person = PostQueryBuilder::create(&conn)
      .listing_type(ListingType::Community)
      .sort(SortType::New)
      .community_id(inserted_community.id)
      .list()
      .unwrap();

    let read_post_listing_no_person = PostView::read(&conn, inserted_post.id, None).unwrap();
    let read_post_listing_with_person =
      PostView::read(&conn, inserted_post.id, Some(inserted_person.id)).unwrap();

    let agg = PostAggregates::read(&conn, inserted_post.id).unwrap();

    // the non person version
    let expected_post_listing_no_person = PostView {
      post: Post {
        id: inserted_post.id,
        name: post_name,
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
        language: PrimaryLanguageTag("en".to_string()),
      },
      my_vote: None,
      creator: PersonSafe {
        id: inserted_person.id,
        name: person_name,
        display_name: None,
        published: inserted_person.published,
        avatar: None,
        actor_id: inserted_person.actor_id.to_owned(),
        local: true,
        admin: false,
        bot_account: false,
        banned: false,
        deleted: false,
        bio: None,
        banner: None,
        updated: None,
        inbox_url: inserted_person.inbox_url.to_owned(),
        shared_inbox_url: None,
        matrix_user_id: None,
      },
      creator_banned_from_community: false,
      community: CommunitySafe {
        id: inserted_community.id,
        name: community_name,
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
      counts: PostAggregates {
        id: agg.id,
        post_id: inserted_post.id,
        comments: 0,
        score: 1,
        upvotes: 1,
        downvotes: 0,
        stickied: false,
        published: agg.published,
        newest_comment_time_necro: inserted_post.published,
        newest_comment_time: inserted_post.published,
      },
      subscribed: false,
      read: false,
      saved: false,
    };

    // TODO More needs to be added here
    let mut expected_post_listing_with_user = expected_post_listing_no_person.to_owned();
    expected_post_listing_with_user.my_vote = Some(1);

    let like_removed = PostLike::remove(&conn, inserted_person.id, inserted_post.id).unwrap();
    let num_deleted = Post::delete(&conn, inserted_post.id).unwrap();
    Community::delete(&conn, inserted_community.id).unwrap();
    Person::delete(&conn, inserted_person.id).unwrap();
    Person::delete(&conn, inserted_bot.id).unwrap();

    // The with user
    assert_eq!(
      expected_post_listing_with_user,
      read_post_listings_with_person[0]
    );
    assert_eq!(
      expected_post_listing_with_user,
      read_post_listing_with_person
    );

    // Should be only one person, IE the bot post should be missing
    assert_eq!(1, read_post_listings_with_person.len());

    // Without the user
    assert_eq!(
      expected_post_listing_no_person,
      read_post_listings_no_person[1]
    );
    assert_eq!(expected_post_listing_no_person, read_post_listing_no_person);

    // Should be 2 posts, with the bot post
    assert_eq!(2, read_post_listings_no_person.len());

    assert_eq!(expected_post_like, inserted_post_like);
    assert_eq!(1, like_removed);
    assert_eq!(1, num_deleted);
  }
}

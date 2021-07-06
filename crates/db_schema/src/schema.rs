table! {
    activity (id) {
        id -> Int4,
        data -> Jsonb,
        local -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        ap_id -> Nullable<Text>,
        sensitive -> Nullable<Bool>,
    }
}

table! {
    comment (id) {
        id -> Int4,
        creator_id -> Int4,
        post_id -> Int4,
        parent_id -> Nullable<Int4>,
        content -> Text,
        removed -> Bool,
        read -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        deleted -> Bool,
        ap_id -> Varchar,
        local -> Bool,
    }
}

table! {
    comment_aggregates (id) {
        id -> Int4,
        comment_id -> Int4,
        score -> Int8,
        upvotes -> Int8,
        downvotes -> Int8,
        published -> Timestamp,
    }
}

table! {
    comment_like (id) {
        id -> Int4,
        person_id -> Int4,
        comment_id -> Int4,
        post_id -> Int4,
        score -> Int2,
        published -> Timestamp,
    }
}

table! {
    comment_report (id) {
        id -> Int4,
        creator_id -> Int4,
        comment_id -> Int4,
        original_comment_text -> Text,
        reason -> Text,
        resolved -> Bool,
        resolver_id -> Nullable<Int4>,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
    }
}

table! {
    comment_saved (id) {
        id -> Int4,
        comment_id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    community (id) {
        id -> Int4,
        name -> Varchar,
        title -> Varchar,
        description -> Nullable<Text>,
        removed -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        deleted -> Bool,
        nsfw -> Bool,
        actor_id -> Varchar,
        local -> Bool,
        private_key -> Nullable<Text>,
        public_key -> Nullable<Text>,
        last_refreshed_at -> Timestamp,
        icon -> Nullable<Varchar>,
        banner -> Nullable<Varchar>,
        followers_url -> Varchar,
        inbox_url -> Varchar,
        shared_inbox_url -> Nullable<Varchar>,
    }
}

table! {
    community_aggregates (id) {
        id -> Int4,
        community_id -> Int4,
        subscribers -> Int8,
        posts -> Int8,
        comments -> Int8,
        published -> Timestamp,
        users_active_day -> Int8,
        users_active_week -> Int8,
        users_active_month -> Int8,
        users_active_half_year -> Int8,
    }
}

table! {
    community_follower (id) {
        id -> Int4,
        community_id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
        pending -> Nullable<Bool>,
    }
}

table! {
    community_moderator (id) {
        id -> Int4,
        community_id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    community_person_ban (id) {
        id -> Int4,
        community_id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    local_user (id) {
        id -> Int4,
        person_id -> Int4,
        password_encrypted -> Text,
        email -> Nullable<Text>,
        show_nsfw -> Bool,
        theme -> Varchar,
        default_sort_type -> Int2,
        default_listing_type -> Int2,
        lang -> Varchar,
        show_avatars -> Bool,
        send_notifications_to_email -> Bool,
        validator_time -> Timestamp,
        show_bot_accounts -> Bool,
        show_scores -> Bool,
        show_read_posts -> Bool,
    }
}

table! {
    mod_add (id) {
        id -> Int4,
        mod_person_id -> Int4,
        other_person_id -> Int4,
        removed -> Nullable<Bool>,
        when_ -> Timestamp,
    }
}

table! {
    mod_add_community (id) {
        id -> Int4,
        mod_person_id -> Int4,
        other_person_id -> Int4,
        community_id -> Int4,
        removed -> Nullable<Bool>,
        when_ -> Timestamp,
    }
}

table! {
    mod_ban (id) {
        id -> Int4,
        mod_person_id -> Int4,
        other_person_id -> Int4,
        reason -> Nullable<Text>,
        banned -> Nullable<Bool>,
        expires -> Nullable<Timestamp>,
        when_ -> Timestamp,
    }
}

table! {
    mod_ban_from_community (id) {
        id -> Int4,
        mod_person_id -> Int4,
        other_person_id -> Int4,
        community_id -> Int4,
        reason -> Nullable<Text>,
        banned -> Nullable<Bool>,
        expires -> Nullable<Timestamp>,
        when_ -> Timestamp,
    }
}

table! {
    mod_lock_post (id) {
        id -> Int4,
        mod_person_id -> Int4,
        post_id -> Int4,
        locked -> Nullable<Bool>,
        when_ -> Timestamp,
    }
}

table! {
    mod_remove_comment (id) {
        id -> Int4,
        mod_person_id -> Int4,
        comment_id -> Int4,
        reason -> Nullable<Text>,
        removed -> Nullable<Bool>,
        when_ -> Timestamp,
    }
}

table! {
    mod_remove_community (id) {
        id -> Int4,
        mod_person_id -> Int4,
        community_id -> Int4,
        reason -> Nullable<Text>,
        removed -> Nullable<Bool>,
        expires -> Nullable<Timestamp>,
        when_ -> Timestamp,
    }
}

table! {
    mod_remove_post (id) {
        id -> Int4,
        mod_person_id -> Int4,
        post_id -> Int4,
        reason -> Nullable<Text>,
        removed -> Nullable<Bool>,
        when_ -> Timestamp,
    }
}

table! {
    mod_sticky_post (id) {
        id -> Int4,
        mod_person_id -> Int4,
        post_id -> Int4,
        stickied -> Nullable<Bool>,
        when_ -> Timestamp,
    }
}

table! {
    password_reset_request (id) {
        id -> Int4,
        token_encrypted -> Text,
        published -> Timestamp,
        local_user_id -> Int4,
    }
}

table! {
    person (id) {
        id -> Int4,
        name -> Varchar,
        display_name -> Nullable<Varchar>,
        avatar -> Nullable<Varchar>,
        banned -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        actor_id -> Varchar,
        bio -> Nullable<Text>,
        local -> Bool,
        private_key -> Nullable<Text>,
        public_key -> Nullable<Text>,
        last_refreshed_at -> Timestamp,
        banner -> Nullable<Varchar>,
        deleted -> Bool,
        inbox_url -> Varchar,
        shared_inbox_url -> Nullable<Varchar>,
        matrix_user_id -> Nullable<Text>,
        admin -> Bool,
        bot_account -> Bool,
    }
}

table! {
    person_aggregates (id) {
        id -> Int4,
        person_id -> Int4,
        post_count -> Int8,
        post_score -> Int8,
        comment_count -> Int8,
        comment_score -> Int8,
    }
}

table! {
    person_ban (id) {
        id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    person_mention (id) {
        id -> Int4,
        recipient_id -> Int4,
        comment_id -> Int4,
        read -> Bool,
        published -> Timestamp,
    }
}

table! {
    post (id) {
        id -> Int4,
        name -> Varchar,
        url -> Nullable<Varchar>,
        body -> Nullable<Text>,
        creator_id -> Int4,
        community_id -> Int4,
        removed -> Bool,
        locked -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        deleted -> Bool,
        nsfw -> Bool,
        stickied -> Bool,
        embed_title -> Nullable<Text>,
        embed_description -> Nullable<Text>,
        embed_html -> Nullable<Text>,
        thumbnail_url -> Nullable<Text>,
        ap_id -> Varchar,
        local -> Bool,
    }
}

table! {
    post_aggregates (id) {
        id -> Int4,
        post_id -> Int4,
        comments -> Int8,
        score -> Int8,
        upvotes -> Int8,
        downvotes -> Int8,
        stickied -> Bool,
        published -> Timestamp,
        newest_comment_time_necro -> Timestamp,
        newest_comment_time -> Timestamp,
    }
}

table! {
    post_like (id) {
        id -> Int4,
        post_id -> Int4,
        person_id -> Int4,
        score -> Int2,
        published -> Timestamp,
    }
}

table! {
    post_read (id) {
        id -> Int4,
        post_id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    post_report (id) {
        id -> Int4,
        creator_id -> Int4,
        post_id -> Int4,
        original_post_name -> Varchar,
        original_post_url -> Nullable<Text>,
        original_post_body -> Nullable<Text>,
        reason -> Text,
        resolved -> Bool,
        resolver_id -> Nullable<Int4>,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
    }
}

table! {
    post_saved (id) {
        id -> Int4,
        post_id -> Int4,
        person_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    private_message (id) {
        id -> Int4,
        creator_id -> Int4,
        recipient_id -> Int4,
        content -> Text,
        deleted -> Bool,
        read -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        ap_id -> Varchar,
        local -> Bool,
    }
}

table! {
    site (id) {
        id -> Int4,
        name -> Varchar,
        sidebar -> Nullable<Text>,
        creator_id -> Int4,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        enable_downvotes -> Bool,
        open_registration -> Bool,
        enable_nsfw -> Bool,
        icon -> Nullable<Varchar>,
        banner -> Nullable<Varchar>,
        description -> Nullable<Text>,
        community_creation_admin_only -> Bool,
    }
}

table! {
    site_aggregates (id) {
        id -> Int4,
        site_id -> Int4,
        users -> Int8,
        posts -> Int8,
        comments -> Int8,
        communities -> Int8,
        users_active_day -> Int8,
        users_active_week -> Int8,
        users_active_month -> Int8,
        users_active_half_year -> Int8,
    }
}

table! {
    person_block (id) {
        id -> Int4,
        person_id -> Int4,
        recipient_id -> Int4,
        published -> Timestamp,
    }
}

table! {
    community_block (id) {
        id -> Int4,
        person_id -> Int4,
        community_id -> Int4,
        published -> Timestamp,
    }
}

// These are necessary since diesel doesn't have self joins / aliases
table! {
    comment_alias_1 (id) {
        id -> Int4,
        creator_id -> Int4,
        post_id -> Int4,
        parent_id -> Nullable<Int4>,
        content -> Text,
        removed -> Bool,
        read -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        deleted -> Bool,
        ap_id -> Varchar,
        local -> Bool,
    }
}

table! {
    person_alias_1 (id) {
        id -> Int4,
        name -> Varchar,
        display_name -> Nullable<Varchar>,
        avatar -> Nullable<Varchar>,
        banned -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        actor_id -> Varchar,
        bio -> Nullable<Text>,
        local -> Bool,
        private_key -> Nullable<Text>,
        public_key -> Nullable<Text>,
        last_refreshed_at -> Timestamp,
        banner -> Nullable<Varchar>,
        deleted -> Bool,
        inbox_url -> Varchar,
        shared_inbox_url -> Nullable<Varchar>,
        matrix_user_id -> Nullable<Text>,
        admin -> Bool,
        bot_account -> Bool,
    }
}

table! {
    person_alias_2 (id) {
        id -> Int4,
        name -> Varchar,
        display_name -> Nullable<Varchar>,
        avatar -> Nullable<Varchar>,
        banned -> Bool,
        published -> Timestamp,
        updated -> Nullable<Timestamp>,
        actor_id -> Varchar,
        bio -> Nullable<Text>,
        local -> Bool,
        private_key -> Nullable<Text>,
        public_key -> Nullable<Text>,
        last_refreshed_at -> Timestamp,
        banner -> Nullable<Varchar>,
        deleted -> Bool,
        inbox_url -> Varchar,
        shared_inbox_url -> Nullable<Varchar>,
        matrix_user_id -> Nullable<Text>,
        admin -> Bool,
        bot_account -> Bool,
    }
}

joinable!(comment_alias_1 -> person_alias_1 (creator_id));
joinable!(comment -> comment_alias_1 (parent_id));
joinable!(person_mention -> person_alias_1 (recipient_id));
joinable!(post -> person_alias_1 (creator_id));
joinable!(comment -> person_alias_1 (creator_id));

joinable!(post_report -> person_alias_2 (resolver_id));
joinable!(comment_report -> person_alias_2 (resolver_id));

joinable!(person_block -> person (person_id));
joinable!(person_block -> person_alias_1 (recipient_id));

joinable!(comment -> person (creator_id));
joinable!(comment -> post (post_id));
joinable!(comment_aggregates -> comment (comment_id));
joinable!(comment_like -> comment (comment_id));
joinable!(comment_like -> person (person_id));
joinable!(comment_like -> post (post_id));
joinable!(comment_report -> comment (comment_id));
joinable!(comment_saved -> comment (comment_id));
joinable!(comment_saved -> person (person_id));
joinable!(community_aggregates -> community (community_id));
joinable!(community_block -> community (community_id));
joinable!(community_block -> person (person_id));
joinable!(community_follower -> community (community_id));
joinable!(community_follower -> person (person_id));
joinable!(community_moderator -> community (community_id));
joinable!(community_moderator -> person (person_id));
joinable!(community_person_ban -> community (community_id));
joinable!(community_person_ban -> person (person_id));
joinable!(local_user -> person (person_id));
joinable!(mod_add_community -> community (community_id));
joinable!(mod_ban_from_community -> community (community_id));
joinable!(mod_lock_post -> person (mod_person_id));
joinable!(mod_lock_post -> post (post_id));
joinable!(mod_remove_comment -> comment (comment_id));
joinable!(mod_remove_comment -> person (mod_person_id));
joinable!(mod_remove_community -> community (community_id));
joinable!(mod_remove_community -> person (mod_person_id));
joinable!(mod_remove_post -> person (mod_person_id));
joinable!(mod_remove_post -> post (post_id));
joinable!(mod_sticky_post -> person (mod_person_id));
joinable!(mod_sticky_post -> post (post_id));
joinable!(password_reset_request -> local_user (local_user_id));
joinable!(person_aggregates -> person (person_id));
joinable!(person_ban -> person (person_id));
joinable!(person_mention -> comment (comment_id));
joinable!(person_mention -> person (recipient_id));
joinable!(post -> community (community_id));
joinable!(post -> person (creator_id));
joinable!(post_aggregates -> post (post_id));
joinable!(post_like -> person (person_id));
joinable!(post_like -> post (post_id));
joinable!(post_read -> person (person_id));
joinable!(post_read -> post (post_id));
joinable!(post_report -> post (post_id));
joinable!(post_saved -> person (person_id));
joinable!(post_saved -> post (post_id));
joinable!(site -> person (creator_id));
joinable!(site_aggregates -> site (site_id));

allow_tables_to_appear_in_same_query!(
  activity,
  comment,
  comment_aggregates,
  community_block,
  comment_like,
  comment_report,
  comment_saved,
  community,
  community_aggregates,
  community_follower,
  community_moderator,
  community_person_ban,
  local_user,
  mod_add,
  mod_add_community,
  mod_ban,
  mod_ban_from_community,
  mod_lock_post,
  mod_remove_comment,
  mod_remove_community,
  mod_remove_post,
  mod_sticky_post,
  password_reset_request,
  person,
  person_aggregates,
  person_ban,
  person_block,
  person_mention,
  post,
  post_aggregates,
  post_like,
  post_read,
  post_report,
  post_saved,
  private_message,
  site,
  site_aggregates,
  comment_alias_1,
  person_alias_1,
  person_alias_2,
);

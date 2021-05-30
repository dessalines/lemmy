ALTER TABLE comment DROP COLUMN language;
ALTER TABLE post DROP COLUMN language;
ALTER TABLE private_message DROP COLUMN language;

ALTER TABLE local_user DROP COLUMN discussion_languages;
ALTER TABLE local_user RENAME COLUMN interface_language TO lang;

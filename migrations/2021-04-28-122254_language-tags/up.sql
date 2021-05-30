ALTER TABLE comment ADD COLUMN language TEXT NOT NULL;
ALTER TABLE post ADD COLUMN language TEXT NOT NULL;
ALTER TABLE private_message ADD COLUMN language TEXT NOT NULL;

ALTER TABLE local_user ADD COLUMN discussion_languages TEXT[] NOT NULL;
ALTER TABLE local_user RENAME COLUMN lang TO interface_language;

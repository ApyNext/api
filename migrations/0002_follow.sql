CREATE TABLE IF NOT EXISTS follow (
  id BIGSERIAL PRIMARY KEY,
  follower_id BIGSERIAL REFERENCES Account(id),
  followed_id BIGSERIAL REFERENCES Account(id),
  CONSTRAINT unique_follow UNIQUE (follower_id, followed_id)
);

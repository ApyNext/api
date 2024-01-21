CREATE TABLE IF NOT EXISTS follow (
  id BIGSERIAL PRIMARY KEY,
  follower_id BIGSERIAL REFERENCES account(id),
  followed_id BIGSERIAL REFERENCES account(id),
  CONSTRAINT unique_follow UNIQUE (follower_id, followed_id)
);

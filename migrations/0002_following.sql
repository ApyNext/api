CREATE TABLE IF NOT EXISTS follow (
  id BIGSERIAL PRIMARY KEY,
  follower_id BIGSERIAL REFERENCES users(id),
  followed_id BIGSERIAL REFERENCES users(id)
);

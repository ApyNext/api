CREATE TABLE IF NOT EXISTS follow (
  follower_id BIGSERIAL REFERENCES account(id),
  followed_id BIGSERIAL REFERENCES account(id),
  PRIMARY KEY (follower_id, followed_id)
);

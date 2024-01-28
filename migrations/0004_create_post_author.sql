-- Your SQL goes here
CREATE TABLE post_author (
    post_id BIGSERIAL REFERENCES post(id),
    author_id BIGSERIAL REFERENCES account(id),
    PRIMARY KEY (post_id, author_id)
)

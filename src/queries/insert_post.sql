WITH inserted_post AS (
    INSERT INTO post (author_id, title, content) VALUES ($1, $2, $3) RETURNING *
)
SELECT inserted_post.id, title, inserted_post.created_at, account.id AS author_id, account.username AS author_username, account.permission AS author_permission
FROM inserted_post
JOIN account ON inserted_post.author_id = account.id;

SELECT post.id, post.title, post.content, post.created_at, post.updated_at, account.id AS author_id, account.username AS author_username, account.permission AS author_permission
FROM post
JOIN account ON post.author_id = account.id
LIMIT $1
OFFSET $2;
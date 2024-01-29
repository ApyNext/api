# api
The official API of ApyNext

**Version française dans le fichier README.md**

# Summary
- [Configuration](#configuration)
- [Launch the API](#launch-the-api)
- [Documentation](#documentation)
    - [Account management](#account-management)
        - [Create an account](#create-an-account)
        - [Email confirmation (link sent paby email)](#email-confirmation-link-sent-by-email)
        - [Login](#login)
        - [A2F (link sent by email)](#a2f-link-sent-by-email)
    - [WebSockets](#websockets)
    - [Follow an user](#follow-an-user)
    - [Post Management](#post-management)
        - [Publish a new post](#publish-a-new-post)
        - [Get posts](#get-posts)

# Configuration
- Configure Postgres on your computer, you can either install it directly (more infos [here](https://www.postgresql.org/docs/15/install-short.html)) - don't forget to set a password for the user postgres - or use the project's docker-compose.yml file :
1) Install Docker (more infos [here](https://www.docker.com/)).
2) Install also Docker compose (more infos [here](https://docs.docker.com/compose/install/)).
3) Run this command by replacing `<password>` by the password you want for the DB :
```bash
POSTGRES_PASSWORD="<password>" docker compose up -d
```
- Rename (or copy) the file .env.example in .env and enter the missing informations

# Launch the API
To launch the API locally you can run
```bash
cargo run
```

# Documentation
## Test route
Request : `GET /`

Returns :
- Status code `200 Ok` and the message "Ok"

## Account management
### Create an account
Request : `POST /register`

Body (JSON) :
- username => string that contains between 5 and 12 characters inclusive, which begins by a letter and can only contain letters, numbers and underscores
- email => string of a valid email
- password => string containing at least 8 characters
- birthdate => Unix timestamp since 1900 to today
- dark_mode => boolean (optional)
- biography => string of less than 300 characters (optional)
- is_male (optional for privacy reasons) => boolean (true for a man and false for a woman)

Returns :
- Status code `200 Ok`
- Status code `400 Bad request` when the body isn't a valid JSON
- Status code `403 Forbidden` and the error message when a client error occurs
- Status code `415 Unsupported Media Type` when the header `Content-Type: application/json` is missing
- Status code `422 Unprocessable Entity` when a JSON field is missing
- Status code `500 Internal Server Error` when a server error occurs

### Email confirmation (link sent by email)
Request : `POST /register/email_confirm`

Body (string) :
- email confirmation token

Returns :
- Status code `200 Ok` and an auth token stored as a cookie
- Status code `403 Forbidden` and the error message when the token is missing, invalid or expired for example
- Status code `500 Internal Server Error` when a server error occurs

### Login
Request : `POST /login`

Body (JSON) :
- username_or_email => string, either :
    - an username containing between 5 and 12 characters included, which begins by a letter and can only contain letters, numbers and underscores
    - a valid email
- password => password (at least 8 characters)

Returns :
- Status code `200 Ok`
- Status code `400 Bad request` when the body isn't a valid JSON
- Status code `403 Forbidden` and the error message when a client error occurs
- Status code `415 Unsupported Media Type` when the header `Content-Type: application/json` is missing
- Status code `422 Unprocessable Entity` when a JSON field is missing
- Status code `500 Internal Server Error` when a server error occurs

### A2F (link sent by email)
Request : `POST /login/a2f`

Body (string) :
- auth verification token

Returns :
- Status code `200 Ok` and an auth token stored as a cookie
- Status code `403 Forbidden` and the error message when the token is missing, invalid or expired

## WebSockets
Requête : `GET /ws`

**More information in doc/websockets_en.md**

## Follow an user
Request : `POST /:id/follow`

Headers :
- Bearer token

Returns :
- Status code `200 Ok`
- Status code `403 Forbidden` with the error message when a client error occurs
- Status code `500 Internal Server Error` when a server error occurs

## Post management
### Publish a new post
Request : `POST /posts/new`

Headers :
- Bearer token

Body (JSON) :
- title => string
- content => string

Returns :
- Status code 200
- Status code `403 Forbidden` with the error message when a client error occurs
- Status code `500 Internal Server Error` when a server error occurs

### Get posts
Request : `GET /posts`

Headers :
- Bearer token (optional)

Query :
- limit => number superior or equal to 0 (optional) -> limit of the posts sent
- offset => number superior or equal to 0 (optional) -> number of posts skipped

Returns :
- Status code `200 Ok`
    Body (JSON) :
    ```json
    [
        {
            "id": <number>, //post id
            "author": {
                "id": <number>, //author id
                "username": <string>, //author username
                "permission": <number>, //author permission (0 = User, 1 = Moderator et 2 = Administrator)
            },
            "title": <string>, //post title
            "content": <string>, //post content
            "created_at": <timestamp UTC>, //post creation date
            "updated_at": <timestamp UTC> //post's last modification date
        }
    ]
    ```
- Status code `500 Internal Server Error` when a server error occurs

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

# Configuration
- Configure Postgres on your computer, you can either install it directly (more infos [here](https://www.postgresql.org/docs/15/install-short.html)) - don't forget to set a password for the user postgres - or use the project's Dockerfile :
1) Install Docker (more infos [here](https://www.docker.com/)).
2) Run this command by replacing `<password>` by the password you want for the DB :
```bash
docker build -t postgres . && docker run -e POSTGRES_PASSWORD="<mot de passe>" -p 5432:5432 postgres
```
- Install Shuttle's CLI, more infos [here](https://docs.shuttle.rs/introduction/installation).
- Install SQLx' CLI, more infos [here](https://docs.rs/crate/sqlx-cli/latest).
- Rename (or copy) the file Secrets.toml.example in Secrets.toml and enter the missing informations

# Launch the API
First of all run the two following commands :
```bash
cargo sqlx migrate run --database-url <DB URL>
```
```bash
cargo sqlx prepare --database-url <DB URLs>
```
Then to launch the API locally you can run
```bash
cargo shuttle run
```
If you want to deploy the API, you can [create a Shuttle account](https://console.shuttle.rs/login) then follow the steps listed [here](https://console.shuttle.rs/new-project) (you can skip the installation of the CLI, you already did it).

# Documentation
## Account management
### Create an account
Request : `POST /register`

Body (JSON) :
- username => string that contains between 5 and 12 characters inclusive, which begins by a letter and can only contain letters, numbers and underscores
- email => string of a valid email
- password => string containing at least 8 characters
- birthdate => Unix timestamp since 1900 to today
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
- Status code `403 Forbidden` and the error message when the token is missing, invalid or expired
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

### A2F (link sent by email)
Request : `POST /login/a2f`

Body (string) :
- auth verification token

Returns :
- Status code `200 Ok` and an auth token stored as a cookie
- Status code `403 Forbidden` and the error message when the token is missing, invalid or expired
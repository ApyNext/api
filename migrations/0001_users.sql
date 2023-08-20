CREATE TABLE IF NOT EXISTS users (
    id  BIGSERIAL PRIMARY KEY,
    username VARCHAR(15) UNIQUE,
    email VARCHAR(320) UNIQUE,
    password VARCHAR(128) NOT NULL,
    birthdate TIMESTAMPTZ NOT NULL,
    dark_mode BOOLEAN NOT NULL DEFAULT TRUE,
    biography VARCHAR(300) NOT NULL DEFAULT '',
    token VARCHAR(256) NOT NULL UNIQUE,
    is_male BOOLEAN,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    email_verified BOOLEAN NOT NULL DEFAULT FALSE
)
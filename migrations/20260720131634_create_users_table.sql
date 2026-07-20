CREATE TABLE users (
    uuid TEXT PRIMARY KEY NOT NULL,
    plex_user_id INTEGER NOT NULL UNIQUE,
    jellyfin_api_key TEXT NOT NULL
);

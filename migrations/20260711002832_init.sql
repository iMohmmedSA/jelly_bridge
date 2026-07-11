CREATE TABLE server_identity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    machine_identifier TEXT NOT NULL,
    plex_auth_token TEXT,
    server_name TEXT NOT NULL DEFAULT 'Jelly Bridge',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

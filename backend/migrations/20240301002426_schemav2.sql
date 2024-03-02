-- Add migration script here
-- Add migration script here

CREATE TABLE User (
    user_id INTEGER PRIMARY KEY NOT NULL,
    public_key TEXT,                            -- for a group dm to be encrypted, users will need to be able to send
                                                -- the encryption key to new members of the group, they can do this using
                                                -- asymmetric encryption to share the symmetric keys.
    user_name TEXT,
    pass_hash TEXT,
    join_date REAL,
    display_name TEXT
);

CREATE TABLE Channel (
    channel_id INTEGER PRIMARY KEY NOT NULL,
    channel_name TEXT,
    community_id INTEGER
);

CREATE TABLE ChannelEvent (
    event_id INTEGER PRIMARY KEY NOT NULL,
    event_datetime REAL,
    channel_id INTEGER NOT NULL,
    FOREIGN KEY (channel_id)
        REFERENCES Channel (channel_id)
);

CREATE TABLE SessionToken ( -- handles authentication - is stored in a cookie
                            -- users are assigned an auth token when logging in
                            -- this token will expire every so often, or can be reset manually
    token_id INTEGER PRIMARY KEY NOT NULL,
    token TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    user_id INTEGER NOT NULL,
    FOREIGN KEY (user_id)
        REFERENCES User (user_id)  
);

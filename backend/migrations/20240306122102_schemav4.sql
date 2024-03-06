-- Add migration script here

DROP TABLE Message;

CREATE TABLE Message (
    message_id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    datetime INTEGER NOT NULL,
    FOREIGN KEY (user_id)
        REFERENCES User (user_id)
);

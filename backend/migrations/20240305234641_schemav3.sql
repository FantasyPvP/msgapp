-- Add migration script here

CREATE TABLE Message (
    user_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    datetime INTEGER NOT NULL,
    FOREIGN KEY (user_id)
        REFERENCES User (user_id)
);

-- Add migration script here

CREATE TABLE newsletter_issues (
    id UUID NOT NULL,
    title TEXT NOT NULL,
    text_content TEXT NOT NULL,
    html_content TEXT NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(id)
);
-- Add migration script here
CREATE TABLE users (
  id UUID PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL 
);
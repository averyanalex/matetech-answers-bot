DROP TABLE answers;
CREATE TABLE answers (
    id BIGSERIAL PRIMARY KEY,
    question TEXT NOT NULL,
    human TEXT NOT NULL,
    exact TEXT NOT NULL,
    machine TEXT NOT NULL
);
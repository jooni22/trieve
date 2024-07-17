-- Your SQL goes here
CREATE TABLE verification_notifications (
    id UUID PRIMARY KEY,
    user_uuid UUID NOT NULL REFERENCES users (id),
    card_uuid UUID NOT NULL REFERENCES card_metadata (id),
    verification_uuid UUID NOT NULL REFERENCES card_verification (id),
    similarity_score BigInt NOT NULL,
    user_read boolean NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

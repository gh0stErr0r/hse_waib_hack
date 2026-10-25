CREATE TABLE gastro_conversations (
 id UUID PRIMARY KEY,
 owner_id TEXT NOT NULL,
 title TEXT NOT NULL DEFAULT 'Новый гастрономический план',
 document JSONB NOT NULL DEFAULT '{}',
 version INTEGER NOT NULL DEFAULT 0,
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX gastro_conversations_owner_updated ON gastro_conversations(owner_id, updated_at DESC, id);

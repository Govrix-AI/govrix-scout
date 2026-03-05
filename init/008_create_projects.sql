-- Migration 008: Create the projects table — groups agents by project/team
--
-- Adds a UUID-keyed projects table and a nullable project_id FK on agents.
-- Backward compatible: existing agents keep project_id = NULL.

CREATE TABLE IF NOT EXISTS projects (
    id          UUID NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add project_id to agents table (nullable = backward compatible)
ALTER TABLE agents ADD COLUMN IF NOT EXISTS project_id UUID REFERENCES projects(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_agents_project_id ON agents(project_id) WHERE project_id IS NOT NULL;

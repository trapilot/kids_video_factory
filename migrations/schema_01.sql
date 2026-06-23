CREATE TABLE `workflows` (
    `id` TEXT PRIMARY KEY,
    `age` INTEGER NOT NULL,
    `task` TEXT NOT NULL,
    `topic` TEXT,
    `status` TEXT NOT NULL,
    `threshold_at` INTEGER,
    `created_at` INTEGER NOT NULL,
    `updated_at` INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS `jobs` (
    `id` TEXT PRIMARY KEY,
    `workflow_id` INTEGER NOT NULL,
    `agent` TEXT NOT NULL,
    `parent` TEXT NOT NULL,
    `version` TEXT NOT NULL,
    `status` TEXT NOT NULL,
    `payload` TEXT NOT NULL,
    `result` TEXT,
    `retry_count` INTEGER DEFAULT 0,
    `max_retry` INTEGER DEFAULT 3,
    `locked_at` INTEGER,
    `threshold_at` INTEGER,
    `created_at` INTEGER NOT NULL,
    `updated_at` INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS `providers` (
    `name` TEXT PRIMARY KEY,
    `running` INTEGER DEFAULT 0,
    `limit` INTEGER DEFAULT 1,
    `blocked_until` INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `oauth_tokens` (
    `provider` TEXT PRIMARY KEY,
    `client_id` TEXT NOT NULL,
    `client_secret` TEXT NOT NULL,
    `access_token` TEXT,
    `refresh_token` TEXT,
    `auth_code` TEXT,
    `expires_at` INTEGER,
    `updated_at` INTEGER NOT NULL
);


CREATE INDEX `idx_workflow_age_status` ON `workflows`(`age`, `status`);

CREATE INDEX `idx_jobs_agent_status` ON `jobs`(`agent`, `status`);

CREATE INDEX `idx_jobs_workflow` ON `jobs`(`workflow_id`);

CREATE INDEX `idx_jobs_parent` ON `jobs`(`parent`);
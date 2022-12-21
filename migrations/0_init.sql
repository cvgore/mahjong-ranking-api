--
-- NOTES:
--  * PRIMARY KEY must have NOT NULL constraint - otherwise sqlx will report Option<TYPE>
--

CREATE TABLE `rankings_cache` (
    `uuid` TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    `name` TEXT NOT NULL,
    `created_at` INTEGER NOT NULL,
    `archived_at` INTEGER NULL
);

-- if tracker_id is set, it means that game is external and stats are not available
CREATE TABLE `game_sessions` (
    `uuid` TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    `ranking_uuid` TEXT NOT NULL COLLATE BINARY,
    `creator_uuid` TEXT NOT NULL COLLATE BINARY,
    `player1_uuid` TEXT NOT NULL COLLATE BINARY,
    `player2_uuid` TEXT NOT NULL COLLATE BINARY,
    `player3_uuid` TEXT NOT NULL COLLATE BINARY,
    `player4_uuid` TEXT NOT NULL COLLATE BINARY,
    `tournament_uuid` TEXT COLLATE BINARY,
    `place_uuid` TEXT NOT NULL COLLATE BINARY,
    `is_shuffled` INTEGER NOT NULL,
    `is_novice_friendly` INTEGER NOT NULL,
    `is_unranked` INTEGER NOT NULL,
    `is_announced` INTEGER NOT NULL,
    `is_player_certified_referee` INTEGER NOT NULL,
    `is_league_game` INTEGER NOT NULL,
    `is_tonpuu` INTEGER NOT NULL,
    `is_too_slow` INTEGER NOT NULL,
    `is_tenant_host` INTEGER NOT NULL,
    `is_hidden` INTEGER NOT NULL,
    `is_not_computed` INTEGER NOT NULL,
    `is_verification_required` INTEGER NOT NULL,
    `is_compute_skipped` INTEGER NOT NULL,
    `created_at` INTEGER NOT NULL
);

CREATE TABLE `game_session_events` (
    `uuid` TEXT NOT NULL COLLATE BINARY,
    `game_session_uuid` TEXT NOT NULL COLLATE BINARY,
    `creator_uuid` TEXT NOT NULL COLLATE BINARY,
    `event_type` TEXT NOT NULL,
    `event_data` TEXT NULL,
    `created_at` INTEGER NOT NULL
);

-- table which stores cached, computed data of game session
-- corresponding score contained within player1_score, player2_score, etc.
-- wind is enum int-indexed => east = 0, south = 1, west = 2, north = 3

CREATE TABLE `game_sessions_stats_cache` (
    `game_session_uuid` TEXT NOT NULL COLLATE BINARY,
    `player1_points` INTEGER NOT NULL,
    `player2_points` INTEGER NOT NULL,
    `player3_points` INTEGER NOT NULL,
    `player4_points` INTEGER NOT NULL,
    `ended_at` INTEGER NOT NULL,
    `round` INTEGER NOT NULL,
    `wind` INTEGER NOT NULL,
    `duration` INTEGER NOT NULL,
    `created_at` INTEGER NOT NULL
);

CREATE TABLE `players_cache` (
    `uuid` TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    `ranking_uuid` TEXT NOT NULL COLLATE BINARY,
    `usma_id` TEXT NOT NULL,
    `first_name` TEXT,
    `last_name` TEXT,
    `city` TEXT,
    `region` TEXT,
    `country_code` TEXT NOT NULL,
    `nickname` TEXT,
    `is_exam_done` INTEGER NOT NULL,
    `is_gdpr_agreed` INTEGER NOT NULL,
    `is_guest` INTEGER NOT NULL,
    `is_static` INTEGER NOT NULL,
    `created_at` INTEGER NOT NULL
);

-- user_uid might be anything (i.e firebase uid, subjectid from jtw, uuid, etc.)
CREATE TABLE `user_player` (
    `user_uid` TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    `player_uuid` TEXT NOT NULL COLLATE BINARY
);

-- place_rank is a computed value from game_sessions daily
CREATE TABLE `places` (
    `uuid` TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    `name` TEXT NOT NULL,
    `street` TEXT,
    `city` TEXT NOT NULL,
    `country_code` TEXT NOT NULL,
    `type` TEXT NOT NULL,
    `rank` INTEGER NOT NULL DEFAULT -1,
    `created_at` INTEGER NOT NULL
);

CREATE UNIQUE INDEX `places_name_uidx` ON `places` (`name` ASC);

CREATE TABLE `ranks_cache` (
    `uuid` TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    `name` TEXT NOT NULL,
    `required_points` INTEGER NOT NULL,
    `required_exam` INTEGER NOT NULL,
    `color` TEXT NOT NULL,
    `created_at` INTEGER NOT NULL
);

CREATE TABLE `ranking_snapshot_cache` (
    `ranking_uuid` TEXT NOT NULL COLLATE BINARY,
    `player_uuid` TEXT NOT NULL COLLATE BINARY,
    `rank_uuid` TEXT NOT NULL COLLATE BINARY,
    `rank_points` INTEGER NOT NULL,
    `elo_points` INTEGER NOT NULL
);
//! Shared `subject_type` allowlists for the generic engagement/host/finding/
//! credential(/note) polymorphic-association tables (`notes`, `attachments`).
//! These must stay in sync with the `note_subject_type`/`attachment_subject_type`
//! Postgres enums (see `backend/migrations/0005_findings_notes.sql` and the
//! narrowing in `0018_remove_observations.sql`) -- kept in one place so a future
//! enum change only needs one Rust-side edit instead of one per route file.

pub const NOTE_SUBJECT_TYPES: [&str; 4] = ["engagement", "host", "finding", "credential"];
pub const ATTACHMENT_SUBJECT_TYPES: [&str; 5] =
    ["engagement", "host", "finding", "credential", "note"];

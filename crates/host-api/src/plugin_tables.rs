//! The `/` mode's command tables and the `@` mode's name list, read from the plugins directory into the Engine options.
//!
//! Both are files the settings page writes while input processes keep running: an imported command-table pack and `mentions.json`. A session remembers the modification time and length of every file it read, checks them again when a field gains focus and when preferences are applied, and reads the files only when something moved. Nothing is read for a mode that is switched off.

use msime_client_core::plugins::{
    command_table, kind_directory, mentions, PluginKind, MANIFEST_FILE,
};
use msime_engine::host::{CommandTableEntry, EngineOptions, MentionEntry};
use std::path::Path;
use std::time::SystemTime;

/// What a file looked like when it was read; `None` when it was not there.
type FileStamp = Option<(Option<SystemTime>, u64)>;

fn stamp(path: &Path) -> FileStamp {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    Some((metadata.modified().ok(), metadata.len()))
}

/// The files a session's Engine options were filled from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PluginTables {
    /// The enabled command-table ids with each manifest's stamp, while the `/` mode is on.
    commands: Option<Vec<(String, FileStamp)>>,
    /// `mentions.json`'s stamp, while the `@` mode is on.
    mentions: Option<FileStamp>,
}

impl PluginTables {
    /// Stamp the files `options` would be filled from. Cheap: a few `stat`s, and none for a mode that is off or a session without a plugins directory.
    pub(crate) fn stamp(root: Option<&Path>, options: &EngineOptions, enabled: &[String]) -> Self {
        let Some(root) = root else {
            return Self::default();
        };
        let tables = kind_directory(root, PluginKind::CommandTable);
        Self {
            commands: options.local_command.then(|| {
                enabled
                    .iter()
                    .map(|id| (id.clone(), stamp(&tables.join(id).join(MANIFEST_FILE))))
                    .collect()
            }),
            mentions: options
                .local_mention
                .then(|| stamp(&mentions::document_path(root))),
        }
    }

    pub(crate) fn commands_differ(&self, previous: &Self) -> bool {
        self.commands != previous.commands
    }

    pub(crate) fn mentions_differ(&self, previous: &Self) -> bool {
        self.mentions != previous.mentions
    }

    /// The merged rows of the enabled command tables; empty while the `/` mode is off.
    pub(crate) fn command_table(&self, root: Option<&Path>) -> Vec<CommandTableEntry> {
        let (Some(root), Some(commands)) = (root, &self.commands) else {
            return Vec::new();
        };
        let enabled: Vec<String> = commands.iter().map(|(id, _)| id.clone()).collect();
        command_table::enabled_commands(root, &enabled)
            .into_iter()
            .map(|row| CommandTableEntry {
                trigger: row.trigger,
                title: row.title,
                template: row.template,
            })
            .collect()
    }

    /// The saved name list; empty while the `@` mode is off or when the document does not load, which the settings page reports. A Chinese name saved without a key is given the reading of the Engine's dictionary in `options`.
    pub(crate) fn mention_entries(
        &self,
        root: Option<&Path>,
        options: &EngineOptions,
    ) -> Vec<MentionEntry> {
        let (Some(root), Some(_)) = (root, &self.mentions) else {
            return Vec::new();
        };
        let entries = mentions::MentionStore::new(root).load().unwrap_or_default();
        entries
            .into_iter()
            .map(|entry| {
                let key = if entry.key.is_empty() && !entry.text.is_ascii() {
                    msime_engine::host::hanzi_to_pinyin(options, &entry.text)
                } else {
                    entry.key
                };
                MentionEntry {
                    text: entry.text,
                    key,
                }
            })
            .collect()
    }

    /// Fill `options` from the files that moved since `previous` was stamped.
    pub(crate) fn fill(&self, previous: &Self, root: Option<&Path>, options: &mut EngineOptions) {
        if self.commands_differ(previous) {
            options.command_table = self.command_table(root);
        }
        if self.mentions_differ(previous) {
            options.mention_entries = self.mention_entries(root, options);
        }
    }
}

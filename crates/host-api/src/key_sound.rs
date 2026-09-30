//! Key sounds, the typing melody, commit and achievement sounds, and background music.
//!
//! One player serves the whole process, however many sessions it holds: there is one audio device, and a host with several fields open types into one of them at a time. It starts on the first call that has something switched on, not when the library loads, so the Windows TSF DLL, which links this library into every process it is injected into and never calls these functions, opens no audio device and starts no thread.
//!
//! The key path only posts a request. A session compares a generation number rather than its settings, the request goes into a bounded queue with `try_send`, so a full queue drops a sound instead of holding up a keystroke, and everything else - resolving packs, decoding, mixing - happens on the player's own thread. Samples are decoded on a thread of their own and the whole table is swapped in when it is ready, so changing packs never plays half of one and half of the other.
//!
//! Only macOS, Windows and Linux play anything here. On iOS, Android and HarmonyOS the entry points report that nothing was queued; HarmonyOS plays packs itself from the files `pack_files` resolves.
//!
//! A failure of the audio device, or a panic anywhere in the player, turns sound off for the rest of the process with one line on standard error. A pack that does not decode is reported once and stays silent.

// Without the player the melody, the volume curve and most of the settings go unread on the platforms that do not play.
#![cfg_attr(
    any(target_os = "ios", target_os = "android", target_env = "ohos"),
    allow(dead_code)
)]

use msime_client_core::plugins::{self, sound_pack, PluginContent, PluginKind};
use msime_client_core::preferences::{KeySoundMode, PluginPreferences};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
mod decode;
#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
mod player;

/// The key classes a host reports, numbered as `msime_client_key_sound` takes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyClass {
    Default = 0,
    Space = 1,
    Enter = 2,
    Backspace = 3,
}

impl KeyClass {
    pub(crate) const ALL: [Self; 4] = [Self::Default, Self::Space, Self::Enter, Self::Backspace];

    pub(crate) fn from_code(code: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|class| *class as u32 == code)
    }

    /// The class name `SoundPack::key_sample` takes.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Space => "space",
            Self::Enter => "enter",
            Self::Backspace => "backspace",
        }
    }
}

/// What a session asks the player for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Key(KeyClass),
    Commit,
    Achievement,
    /// Whether the input method is the active one in a field that may hear music: false when it loses activation or the focused field is a secure one.
    Music(bool),
}

/// The directories a session reads plugin packs from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PluginRoots {
    /// `<state root>/plugins`: installed packs and `mentions.json`. `None` when the host named no state root.
    pub installed: Option<PathBuf>,
    /// The built-in sound packs of the host's bundle.
    pub builtin_sounds: Option<PathBuf>,
}

impl PluginRoots {
    /// `state_root` is HostOptions' `preferences_directory`, `sound_packs` its `sound_packs`. Without `sound_packs` the built-in packs are looked for in `sound-packs` beside `resources`, where the macOS bundle and the Linux install put them; a relative path names nothing.
    pub(crate) fn new(
        state_root: Option<&str>,
        sound_packs: Option<&str>,
        resources: &str,
    ) -> Self {
        let absolute = |path: &str| {
            let path = Path::new(path);
            path.is_absolute().then(|| path.to_path_buf())
        };
        let builtin_sounds = match sound_packs {
            Some(path) => absolute(path),
            None => absolute(resources)
                .and_then(|resources| Some(resources.parent()?.join("sound-packs")))
                .filter(|path| path.is_dir()),
        };
        Self {
            installed: state_root
                .and_then(absolute)
                .map(|root| root.join("plugins")),
            builtin_sounds,
        }
    }
}

/// The effect and music settings of one preference document, with where their packs are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SoundSettings {
    pub roots: PluginRoots,
    pub key: bool,
    /// Keys play the melody pack's next note instead of their own sample.
    pub melody: bool,
    pub commit: bool,
    pub achievements: bool,
    pub pack: String,
    pub melody_pack: String,
    /// 0-100, for every effect sound.
    pub volume: u8,
    pub music: bool,
    pub music_pack: String,
    pub music_volume: u8,
}

impl SoundSettings {
    pub(crate) fn new(preferences: &PluginPreferences, roots: &PluginRoots) -> Self {
        Self {
            roots: roots.clone(),
            key: preferences.key_sound.enabled,
            melody: preferences.key_sound.mode == KeySoundMode::Melody,
            commit: preferences.commit_sound.enabled,
            achievements: preferences.achievements.enabled,
            pack: preferences.key_sound.pack.clone(),
            melody_pack: preferences.melody.pack.clone(),
            volume: preferences.key_sound.volume,
            music: preferences.music.enabled && !preferences.music.pack.is_empty(),
            music_pack: preferences.music.pack.clone(),
            music_volume: preferences.music.volume,
        }
    }

    /// Whether the player has anything to do. Nothing starts it until this holds, and it lets the audio device go once this stops holding.
    pub(crate) fn wanted(&self) -> bool {
        self.key || self.commit || self.achievements || self.music
    }

    /// Whether a commit makes a sound: its own sample, or the next note of a melody that advances on commits (only the pack knows which, so any melody counts here).
    fn commit_sounds(&self) -> bool {
        self.commit || (self.key && self.melody)
    }
}

/// Numbers every distinct settings value a session publishes, so the key path compares one integer.
static GENERATIONS: AtomicU64 = AtomicU64::new(1);
/// The settings the process's sessions published last, for the calls that come without a session: an achievement is noticed where typing statistics are recorded.
static LATEST: Mutex<Option<(Arc<SoundSettings>, u64)>> = Mutex::new(None);
/// `LATEST`'s achievement switch, read without the lock by every statistics record.
static ACHIEVEMENTS: AtomicBool = AtomicBool::new(false);

/// A session's settings and the generation that names them.
#[derive(Clone, Debug)]
pub(crate) struct SessionSound {
    settings: Arc<SoundSettings>,
    generation: u64,
}

impl SessionSound {
    pub(crate) fn new(settings: SoundSettings) -> Self {
        let sound = Self {
            settings: Arc::new(settings),
            generation: GENERATIONS.fetch_add(1, Ordering::Relaxed),
        };
        sound.publish();
        sound
    }

    /// Take the settings of a newer preference document. The generation only moves when they differ, so a change to an unrelated preference never reloads a pack.
    pub(crate) fn update(&mut self, settings: SoundSettings) {
        if *self.settings != settings {
            *self = Self::new(settings);
        }
    }

    fn publish(&self) {
        ACHIEVEMENTS.store(self.settings.achievements, Ordering::Relaxed);
        *LATEST.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((Arc::clone(&self.settings), self.generation));
    }
}

/// Queue the sound of one key. False when nothing was queued: the key sound is off, this platform does not play, sound failed earlier in this process, or the queue is full.
pub(crate) fn key(sound: &SessionSound, class: KeyClass) -> bool {
    if !sound.settings.key {
        sync(sound);
        return false;
    }
    deliver(sound, Event::Key(class))
}

/// Queue the sound of a commit: the pack's commit sample, the melody's next note, or both.
pub(crate) fn commit(sound: &SessionSound) -> bool {
    if !sound.settings.commit_sounds() {
        sync(sound);
        return false;
    }
    deliver(sound, Event::Commit)
}

/// Tell the player whether music may play now. A running player hears it even while music is off, so music switched on later starts only once the host says the input method is active; with nothing switched on, no player is started for it.
pub(crate) fn music_active(sound: &SessionSound, active: bool) -> bool {
    deliver(sound, Event::Music(active))
}

/// Whether the process's sessions last asked for achievement sounds; the statistics recorder only reads the count before a commit when they did.
pub(crate) fn achievements_armed() -> bool {
    ACHIEVEMENTS.load(Ordering::Relaxed)
}

/// Queue the achievement sample under the settings sessions published last.
pub(crate) fn achievement() -> bool {
    let latest = LATEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let Some((settings, generation)) = latest else {
        return false;
    };
    if !settings.achievements {
        return false;
    }
    deliver(
        &SessionSound {
            settings,
            generation,
        },
        Event::Achievement,
    )
}

#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
fn deliver(sound: &SessionSound, event: Event) -> bool {
    player::deliver(sound, event)
}

#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
fn sync(sound: &SessionSound) {
    player::sync(sound);
}

#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos"))]
fn deliver(_sound: &SessionSound, _event: Event) -> bool {
    false
}

#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos"))]
fn sync(_sound: &SessionSound) {}

/// Steps through a melody note by note.
#[derive(Debug, Default)]
pub(crate) struct Melody {
    next: usize,
    last: Option<Instant>,
}

impl Melody {
    /// The semitone of the next note. The tune starts over at its end, and after `MELODY_IDLE_RESET_MILLIS` without a note, so a pause in typing begins it again.
    pub(crate) fn step(&mut self, semitones: &[i8], now: Instant) -> Option<i8> {
        if semitones.is_empty() {
            return None;
        }
        let idle = Duration::from_millis(sound_pack::MELODY_IDLE_RESET_MILLIS);
        if self.next >= semitones.len()
            || self
                .last
                .is_some_and(|last| now.saturating_duration_since(last) >= idle)
        {
            self.next = 0;
        }
        self.last = Some(now);
        let note = semitones[self.next];
        self.next += 1;
        Some(note)
    }
}

/// A 0-100 volume in decibels. The setting scales amplitude, so 50 is -6 dB; 0 and anything at or below kira's silence floor of -60 dB is silent.
pub(crate) fn decibels(volume: u8) -> f32 {
    const SILENCE: f32 = -60.0;
    if volume == 0 {
        return SILENCE;
    }
    (20.0 * (f32::from(volume.min(100)) / 100.0).log10()).max(SILENCE)
}

/// The files of one validated sound pack as absolute paths, for a host that plays packs itself. Validation is client-core's, the same `scan` lists the pack by, so a host never opens a file the manifest does not name.
pub(crate) fn pack_files(roots: &PluginRoots, id: &str) -> Result<Value, String> {
    let installed = roots.installed.as_deref().unwrap_or(Path::new(""));
    let package = plugins::load_package(
        installed,
        roots.builtin_sounds.as_deref(),
        PluginKind::Sound,
        id,
    )?;
    let PluginContent::Sound(pack) = &package.content else {
        return Err("not a sound pack".into());
    };
    let path = |name: &Option<String>| {
        name.as_ref()
            .map(|name| package.directory.join(name).to_string_lossy().into_owned())
    };
    let sounds = &pack.sounds;
    Ok(json!({
        "id": package.id,
        "name": package.name,
        "license": package.license,
        "builtin": package.builtin,
        "mode": pack.mode,
        "sounds": {
            "default": path(&sounds.default),
            "space": path(&sounds.space),
            "enter": path(&sounds.enter),
            "backspace": path(&sounds.backspace),
            "commit": path(&sounds.commit),
            "achievement": path(&sounds.achievement),
        },
        "sequence": pack.sequence.as_ref().map(|sequence| json!({
            "sample": path(&Some(sequence.sample.clone())),
            "semitones": sequence.semitones,
            "advance": sequence.advance,
        })),
        "max_sample_millis": sound_pack::MAX_SAMPLE_MILLIS,
        "melody_idle_reset_millis": sound_pack::MELODY_IDLE_RESET_MILLIS,
    }))
}

#[cfg(test)]
mod tests;

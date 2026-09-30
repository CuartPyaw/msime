//! The process's player: one thread that owns the kira mixer, and the queue sessions post to.

use super::{decibels, decode, Event, KeyClass, Melody, PluginRoots, SessionSound, SoundSettings};
use kira::sound::static_sound::StaticSoundData;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::PlaybackState;
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Semitones, Tween};
use msime_client_core::plugins::sound_pack::{SequenceAdvance, SoundPack};
use msime_client_core::plugins::{self, PluginContent, PluginKind, PluginSummary};
use std::collections::HashMap;
use std::convert::Infallible;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Requests the queue holds before `try_send` starts dropping sounds. A burst of keys well past what anyone types.
const QUEUE: usize = 64;
/// A key or commit sound older than this when the player gets to it is dropped: late is worse than silent.
const STALE: Duration = Duration::from_millis(200);
/// The audio device is let go after this long without a sound and without music playing. An open output stream keeps the machine from idling (CoreAudio holds a power assertion for it), so a user who switched key sounds on and walked away must not pay for that.
const IDLE_CLOSE: Duration = Duration::from_secs(30);
/// How often the player looks at a playing track, to start the next one when it ends.
const MUSIC_TICK: Duration = Duration::from_millis(250);

type LoadResult = thread::Result<Result<Samples, String>>;

pub(super) enum Request {
    Configure(Arc<SoundSettings>),
    Event(Event, Instant),
    Loaded(u64, Box<LoadResult>),
}

struct Player {
    sender: SyncSender<Request>,
    /// The generation of the settings last sent, so the key path sends settings only when they changed.
    configured: AtomicU64,
}

static PLAYER: OnceLock<Option<Player>> = OnceLock::new();
static DISABLED: AtomicBool = AtomicBool::new(false);

/// Turn sound off for the rest of the process, saying why once.
fn disable(reason: &str) {
    if !DISABLED.swap(true, Ordering::AcqRel) {
        eprintln!("msime: sound is off for the rest of this process: {reason}");
    }
}

/// The running player, started first when `start` is set. `None` once sound has been turned off.
fn player(start: bool) -> Option<&'static Player> {
    if DISABLED.load(Ordering::Acquire) {
        return None;
    }
    let player = if start {
        PLAYER.get_or_init(spawn)
    } else {
        PLAYER.get()?
    };
    player.as_ref()
}

fn spawn() -> Option<Player> {
    let (sender, receiver) = sync_channel(QUEUE);
    let loads = sender.clone();
    match thread::Builder::new()
        .name("msime-sound".into())
        .spawn(move || run(receiver, loads))
    {
        Ok(_) => Some(Player {
            sender,
            configured: AtomicU64::new(0),
        }),
        Err(error) => {
            disable(&format!("the sound thread did not start: {error}"));
            None
        }
    }
}

/// Send the session's settings when the player last saw another generation. False when the queue refused them.
fn configure(player: &Player, sound: &SessionSound) -> bool {
    if player.configured.swap(sound.generation, Ordering::AcqRel) == sound.generation {
        return true;
    }
    let sent = player
        .sender
        .try_send(Request::Configure(Arc::clone(&sound.settings)))
        .is_ok();
    if !sent {
        player.configured.store(0, Ordering::Release);
    }
    sent
}

pub(super) fn deliver(sound: &SessionSound, event: Event) -> bool {
    let Some(player) = player(sound.settings.wanted()) else {
        return false;
    };
    configure(player, sound)
        && player
            .sender
            .try_send(Request::Event(event, Instant::now()))
            .is_ok()
}

/// Bring a running player up to the session's settings without asking for a sound, so switching everything off lets the device go and stops the music.
pub(super) fn sync(sound: &SessionSound) {
    if let Some(player) = player(false) {
        configure(player, sound);
    }
}

fn run(receiver: Receiver<Request>, loads: SyncSender<Request>) {
    let mut worker = Worker::new(loads);
    loop {
        let request = match worker.timeout(Instant::now()) {
            None => match receiver.recv() {
                Ok(request) => Some(request),
                Err(_) => return,
            },
            Some(timeout) => match receiver.recv_timeout(timeout) {
                Ok(request) => Some(request),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
        };
        let handled = catch_unwind(AssertUnwindSafe(|| {
            if let Some(request) = request {
                worker.handle(request);
            }
            worker.tick(Instant::now());
        }));
        if handled.is_err() {
            disable("the sound thread panicked");
        }
        if DISABLED.load(Ordering::Acquire) {
            return;
        }
    }
}

/// The decoded samples of the selected packs.
#[derive(Default)]
pub(super) struct Samples {
    keys: [Option<StaticSoundData>; 4],
    commit: Option<StaticSoundData>,
    achievement: Option<StaticSoundData>,
    melody: Option<MelodySamples>,
}

struct MelodySamples {
    sample: StaticSoundData,
    semitones: Vec<i8>,
    advance: SequenceAdvance,
}

/// What `Samples` are decoded from: the key pack when a key, commit or achievement sound uses it, and the melody pack when keys play the melody.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Selection {
    roots: PluginRoots,
    pack: Option<String>,
    melody_pack: Option<String>,
}

impl Selection {
    pub(super) fn of(settings: &SoundSettings) -> Option<Self> {
        let pack = ((settings.key && !settings.melody) || settings.commit || settings.achievements)
            .then(|| settings.pack.clone());
        let melody_pack = (settings.key && settings.melody).then(|| settings.melody_pack.clone());
        (pack.is_some() || melody_pack.is_some()).then(|| Self {
            roots: settings.roots.clone(),
            pack,
            melody_pack,
        })
    }
}

/// A validated pack of `kind`, installed or (for sound packs) built in.
fn resolve(roots: &PluginRoots, kind: PluginKind, id: &str) -> Result<PluginSummary, String> {
    let installed = roots.installed.as_deref().unwrap_or(Path::new(""));
    plugins::load_package(installed, roots.builtin_sounds.as_deref(), kind, id)
        .map_err(|reason| format!("{} pack {id}: {reason}", kind.as_str()))
}

fn sound_pack(roots: &PluginRoots, id: &str) -> Result<(PathBuf, SoundPack), String> {
    let package = resolve(roots, PluginKind::Sound, id)?;
    match package.content {
        PluginContent::Sound(pack) => Ok((package.directory, pack)),
        _ => Err(format!("sound pack {id}: not a sound pack")),
    }
}

/// Decode every sample `selection` needs, each file once.
pub(super) fn load(selection: &Selection) -> Result<Samples, String> {
    let mut decoded: HashMap<PathBuf, StaticSoundData> = HashMap::new();
    let mut sample = |directory: &Path, name: &str| -> Result<StaticSoundData, String> {
        let path = directory.join(name);
        if let Some(data) = decoded.get(&path) {
            return Ok(data.clone());
        }
        let data = decode::sample(&path).map_err(|reason| format!("{name}: {reason}"))?;
        decoded.insert(path, data.clone());
        Ok(data)
    };
    let mut samples = Samples::default();
    if let Some(id) = &selection.pack {
        let (directory, pack) = sound_pack(&selection.roots, id)?;
        for class in KeyClass::ALL {
            if let Some(name) = pack.key_sample(class.name()) {
                samples.keys[class as usize] = Some(sample(&directory, name)?);
            }
        }
        if let Some(name) = &pack.sounds.commit {
            samples.commit = Some(sample(&directory, name)?);
        }
        if let Some(name) = &pack.sounds.achievement {
            samples.achievement = Some(sample(&directory, name)?);
        }
    }
    if let Some(id) = &selection.melody_pack {
        let (directory, pack) = sound_pack(&selection.roots, id)?;
        let sequence = pack
            .sequence
            .ok_or_else(|| format!("sound pack {id}: not a melody"))?;
        samples.melody = Some(MelodySamples {
            sample: sample(&directory, &sequence.sample)?,
            semitones: sequence.semitones,
            advance: sequence.advance,
        });
    }
    Ok(samples)
}

/// The music pack being played and where in it.
#[derive(Default)]
struct Music {
    /// Whether the host says music may play now.
    active: bool,
    /// The pack's directory and tracks, once resolved for the current settings.
    tracks: Option<(PathBuf, Vec<String>)>,
    resolved: bool,
    track: usize,
    handle: Option<StreamingSoundHandle<Infallible>>,
    /// Raised by the playing track's decoder when it has nothing more to give.
    ended: Arc<AtomicBool>,
}

impl Music {
    /// Forget the pack and stop its track, for a change of pack or a closed device.
    fn reset(&mut self) {
        if let Some(mut handle) = self.handle.take() {
            handle.stop(Tween::default());
        }
        self.tracks = None;
        self.resolved = false;
        self.track = 0;
    }

    fn playing(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| handle.state().is_advancing())
    }
}

pub(super) struct Worker {
    settings: Arc<SoundSettings>,
    loads: SyncSender<Request>,
    /// The load whose result is wanted; results of an earlier one are dropped.
    load: u64,
    selection: Option<Selection>,
    samples: Option<Samples>,
    melody: Melody,
    manager: Option<AudioManager>,
    last_sound: Option<Instant>,
    music: Music,
}

impl Worker {
    pub(super) fn new(loads: SyncSender<Request>) -> Self {
        Self {
            settings: Arc::default(),
            loads,
            load: 0,
            selection: None,
            samples: None,
            melody: Melody::default(),
            manager: None,
            last_sound: None,
            music: Music::default(),
        }
    }

    /// How long the thread may wait for the next request before `tick` has something to do: never while idle with the device closed.
    fn timeout(&self, now: Instant) -> Option<Duration> {
        if self.music.handle.is_some() && self.music.active {
            return Some(MUSIC_TICK);
        }
        self.manager.as_ref()?;
        let since = self
            .last_sound
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        Some(IDLE_CLOSE.saturating_sub(since) + MUSIC_TICK)
    }

    pub(super) fn handle(&mut self, request: Request) {
        match request {
            Request::Configure(settings) => self.configure(settings),
            Request::Event(Event::Music(active), _) => self.music.active = active,
            Request::Event(event, at) => {
                let now = Instant::now();
                if matches!(event, Event::Key(_) | Event::Commit)
                    && now.saturating_duration_since(at) > STALE
                {
                    return;
                }
                for sound in self.sounds_for(event, now) {
                    self.play(sound);
                }
            }
            Request::Loaded(load, result) => self.loaded(load, result),
        }
    }

    pub(super) fn configure(&mut self, settings: Arc<SoundSettings>) {
        if *settings == *self.settings {
            return;
        }
        let selection = Selection::of(&settings);
        if selection != self.selection {
            self.samples = None;
            self.melody = Melody::default();
            self.load += 1;
            self.selection = selection.clone();
            if let Some(selection) = selection {
                let loads = self.loads.clone();
                let load = self.load;
                let spawned = thread::Builder::new()
                    .name("msime-sound-load".into())
                    .spawn(move || {
                        let result = catch_unwind(|| self::load(&selection));
                        // The player has stopped when this fails, and then nobody wants the samples.
                        let _ = loads.send(Request::Loaded(load, Box::new(result)));
                    });
                if let Err(error) = spawned {
                    disable(&format!("the sound pack loader did not start: {error}"));
                }
            }
        }
        let previous = std::mem::replace(&mut self.settings, settings);
        let settings = &self.settings;
        if (previous.music, &previous.music_pack, &previous.roots)
            != (settings.music, &settings.music_pack, &settings.roots)
        {
            self.music.reset();
        } else if previous.music_volume != settings.music_volume {
            if let Some(handle) = &mut self.music.handle {
                handle.set_volume(decibels(settings.music_volume), Tween::default());
            }
        }
        if !settings.wanted() {
            self.close();
        }
    }

    pub(super) fn loaded(&mut self, load: u64, result: Box<LoadResult>) {
        if load != self.load {
            return;
        }
        match *result {
            Ok(Ok(samples)) => self.samples = Some(samples),
            Ok(Err(reason)) => eprintln!("msime: sound pack not loaded: {reason}"),
            Err(_) => disable("decoding a sound pack panicked"),
        }
    }

    /// What `event` plays under the current settings and samples. Stepping the melody is the only state it changes.
    pub(super) fn sounds_for(&mut self, event: Event, now: Instant) -> Vec<StaticSoundData> {
        let settings = &self.settings;
        let Some(samples) = &self.samples else {
            return Vec::new();
        };
        let volume = Decibels(decibels(settings.volume));
        let melody_advance = samples
            .melody
            .as_ref()
            .filter(|_| settings.key && settings.melody)
            .map(|melody| melody.advance);
        let mut sounds = Vec::with_capacity(2);
        let mut note = |melody: &MelodySamples| {
            self.melody
                .step(&melody.semitones, now)
                .map(|semitone| melody.sample.playback_rate(Semitones(f64::from(semitone))))
        };
        match event {
            Event::Key(class) if settings.key => {
                if settings.melody {
                    if melody_advance == Some(SequenceAdvance::Key) {
                        sounds.extend(samples.melody.as_ref().and_then(&mut note));
                    }
                } else {
                    sounds.extend(samples.keys[class as usize].clone());
                }
            }
            Event::Commit => {
                if settings.commit {
                    sounds.extend(samples.commit.clone());
                }
                if melody_advance == Some(SequenceAdvance::Commit) {
                    sounds.extend(samples.melody.as_ref().and_then(&mut note));
                }
            }
            Event::Achievement if settings.achievements => {
                sounds.extend(samples.achievement.clone());
            }
            _ => {}
        }
        sounds
            .into_iter()
            .map(|sound| sound.volume(volume))
            .collect()
    }

    fn manager(&mut self) -> Option<&mut AudioManager> {
        if self.manager.is_none() {
            match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
                Ok(manager) => self.manager = Some(manager),
                Err(error) => {
                    disable(&format!("no audio output: {error}"));
                    return None;
                }
            }
        }
        self.manager.as_mut()
    }

    fn play(&mut self, sound: StaticSoundData) {
        let Some(manager) = self.manager() else {
            return;
        };
        // Past kira's limit of sounds at once, a key sound is simply not heard.
        let _ = manager.play(sound);
        self.last_sound = Some(Instant::now());
    }

    /// Let the audio device go. Music starts its current track again when it next plays.
    fn close(&mut self) {
        self.music.handle = None;
        self.manager = None;
    }

    fn tick(&mut self, now: Instant) {
        self.tick_music(now);
        if self.manager.is_some()
            && !self.music.playing()
            && self
                .last_sound
                .is_none_or(|last| now.saturating_duration_since(last) >= IDLE_CLOSE)
        {
            self.close();
        }
    }

    fn tick_music(&mut self, now: Instant) {
        if !(self.settings.music && self.music.active) {
            if let Some(handle) = &mut self.music.handle {
                if !matches!(
                    handle.state(),
                    PlaybackState::Paused | PlaybackState::Pausing | PlaybackState::Stopped
                ) {
                    handle.pause(Tween::default());
                    self.last_sound = Some(now);
                }
            }
            return;
        }
        if let Some(handle) = &mut self.music.handle {
            match handle.state() {
                PlaybackState::Stopped => {
                    self.music.handle = None;
                    if let Some((_, tracks)) = &self.music.tracks {
                        self.music.track = (self.music.track + 1) % tracks.len();
                    }
                }
                PlaybackState::Paused | PlaybackState::Pausing => {
                    handle.resume(Tween::default());
                }
                PlaybackState::Stopping => {}
                _ if self.music.ended.load(Ordering::Acquire) => handle.stop(Tween::default()),
                _ => {}
            }
            if self.music.handle.is_some() {
                self.last_sound = Some(now);
                return;
            }
        }
        if !self.music.resolved {
            self.music.resolved = true;
            let settings = Arc::clone(&self.settings);
            match resolve(&settings.roots, PluginKind::Music, &settings.music_pack) {
                Ok(PluginSummary {
                    directory,
                    content: PluginContent::Music(pack),
                    ..
                }) => self.music.tracks = Some((directory, pack.tracks)),
                Ok(_) => {}
                Err(reason) => eprintln!("msime: music not played: {reason}"),
            }
        }
        let Some((directory, tracks)) = self.music.tracks.clone() else {
            return;
        };
        let volume = decibels(self.settings.music_volume);
        for _ in 0..tracks.len() {
            let ended = Arc::new(AtomicBool::new(false));
            let path = directory.join(&tracks[self.music.track]);
            if let Ok(decoder) = decode::track(&path, Arc::clone(&ended)) {
                let data = StreamingSoundData::from_decoder(decoder).volume(volume);
                let Some(manager) = self.manager() else {
                    return;
                };
                if let Ok(handle) = manager.play(data) {
                    self.music.ended = ended;
                    self.music.handle = Some(handle);
                    self.last_sound = Some(now);
                    return;
                }
            }
            self.music.track = (self.music.track + 1) % tracks.len();
        }
        eprintln!(
            "msime: music not played: music pack {}: no track could be played",
            self.settings.music_pack
        );
        self.music.tracks = None;
    }
}

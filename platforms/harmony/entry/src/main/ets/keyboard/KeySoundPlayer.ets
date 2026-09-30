/**
 * Key sounds, the typing melody, commit and achievement sounds on a 2in1, played through SoundPool.
 *
 * The desktop hosts play these through host-api, which does not link an audio stack on HarmonyOS, so this host plays the same packs itself. What a pack holds comes from `msime_client_key_sound_pack`, which answers with the files client-core's validation accepted; which of them an event plays is `KeySoundPolicy`, the desktop player's rules. A WAV sample is decoded natively (`keySoundRenderNotes`, miniaudio) under the pack's length bound and written back out once per pitch it is played at, so SoundPool only ever decodes files this host wrote, and a melody's notes are pitched the way the desktop pitches them. An Ogg sample has its length read by the media service first and is played as it is: nothing here decodes Vorbis, so an Ogg melody plays every note at the sample's own pitch.
 *
 * Nothing is created until a sound is switched on, and nothing here can stop a key from typing. A pack that will not load says so once and stays silent; SoundPool failing to play turns the sounds off until the settings change. The pool mixes with whatever else is playing rather than pausing it: SoundPool plays short sounds in mixing mode for the music stream usage, unlike the voice recording's CONCURRENCY_PAUSE_OTHERS session.
 */
import audio from '@ohos.multimedia.audio';
import media from '@ohos.multimedia.media';
import fs from '@ohos.file.fs';
import common from '@ohos.app.ability.common';
import client from 'libmsimeclient.so';
import { KeyboardLog, describeError } from './KeyboardLog';
import {
  KEY_SOUNDS_OFF, KeySoundCue, KeySoundEvent, KeySoundMelody, KeySoundPackFiles, KeySoundPolicy,
  KeySoundSampleRequest, KeySoundSelection, KeySoundSettings
} from './KeySoundPolicy';

interface KeySoundPackRequest {
  state_root: string;
  sound_packs: string;
  pack: string;
}

interface KeySoundPackResponse {
  ok: boolean;
  value: KeySoundPackFiles;
  error: string;
}

// Key sounds are short; eight at once covers fast typing with a melody note still ringing.
const POOL_STREAMS: number = 8;

export class KeySoundPlayer {
  private settings: KeySoundSettings = KEY_SOUNDS_OFF;
  private selection: KeySoundSelection = KeySoundPolicy.selection(KEY_SOUNDS_OFF);
  private stateRoot: string = '';
  private soundPacks: string = '';
  private cacheRoot: string = '';
  /** Moves on every reload, so a load that a newer settings change overtook throws its work away. */
  private generation: number = 0;
  private pool: media.SoundPool | undefined = undefined;
  private keys: KeySoundPackFiles | null = null;
  private melody: KeySoundPackFiles | null = null;
  private readonly melodyState: KeySoundMelody = new KeySoundMelody();
  /** Cue keys (`KeySoundPolicy.cueKey`) of the sounds SoundPool has finished loading. */
  private readonly ready: Map<string, number> = new Map<string, number>();
  /** Loaded files waiting for SoundPool's loadComplete, with the descriptor that has to stay open until then. */
  private readonly loading: Map<number, fs.File> = new Map<number, fs.File>();
  private readonly loadingCues: Map<number, string[]> = new Map<number, string[]>();
  /** loadComplete can arrive before load() resolves with the same id. */
  private readonly completed: Set<number> = new Set<number>();
  private failed: boolean = false;

  /** Where the built-in packs are and where rendered notes may be written. */
  use(context: common.Context): void {
    this.soundPacks = `${context.resourceDir}/sound-packs`;
    this.cacheRoot = `${context.cacheDir}/key-sounds`;
  }

  /**
   * Take the sound settings of a newly read preference document. Only a change of pack reloads anything; the volume is read at every sound.
   *
   * @param stateRoot the preferences directory, which holds the installed packs under `plugins/`
   */
  configure(settings: KeySoundSettings, stateRoot: string): void {
    const selection: KeySoundSelection = KeySoundPolicy.selection(settings);
    const reload: boolean = selection.pack !== this.selection.pack
      || selection.melodyPack !== this.selection.melodyPack || stateRoot !== this.stateRoot;
    this.settings = settings;
    if (!reload) {
      return;
    }
    this.selection = selection;
    this.stateRoot = stateRoot;
    this.release();
    this.failed = false;
    if (!KeySoundPolicy.wanted(settings) || this.soundPacks.length === 0) {
      return;
    }
    const generation: number = this.generation;
    // Off the caller's turn: reading a pack touches the disk, and this runs while the keyboard is starting.
    setTimeout((): void => {
      this.load(generation).catch((error: Object): void => {
        KeyboardLog.error(`key sounds unavailable: ${describeError(error)}`);
      });
    }, 0);
  }

  /** A key went down; `keyClass` is a `KeySoundClass`. */
  key(keyClass: number): void {
    this.play(KeySoundEvent.KEY, keyClass);
  }

  /** Text was committed. */
  commit(): void {
    this.play(KeySoundEvent.COMMIT, 0);
  }

  /** The commit count just passed a milestone. */
  achievement(): void {
    this.play(KeySoundEvent.ACHIEVEMENT, 0);
  }

  /** Let go of the pool, its sounds and the rendered notes. The settings stay, so the next configure with a different pack loads again. */
  release(): void {
    this.generation += 1;
    this.melodyState.reset();
    this.ready.clear();
    this.loadingCues.clear();
    this.completed.clear();
    this.loading.forEach((file: fs.File): void => {
      fs.closeSync(file);
    });
    this.loading.clear();
    this.keys = null;
    this.melody = null;
    const pool: media.SoundPool | undefined = this.pool;
    this.pool = undefined;
    if (pool !== undefined) {
      pool.release().catch((error: Object): void => {
        KeyboardLog.error(`key sound pool would not release: ${describeError(error)}`);
      });
    }
    this.clearNotes();
  }

  /** Stop for good: the ability is going away. */
  shutdown(): void {
    this.release();
    this.settings = KEY_SOUNDS_OFF;
    this.selection = KeySoundPolicy.selection(KEY_SOUNDS_OFF);
  }

  private play(event: KeySoundEvent, keyClass: number): void {
    const pool: media.SoundPool | undefined = this.pool;
    if (pool === undefined || this.failed) {
      return;
    }
    const cues: KeySoundCue[] = KeySoundPolicy.cues(this.settings, this.keys, this.melody, event,
      keyClass, this.melodyState, Date.now());
    const gain: number = KeySoundPolicy.gain(this.settings);
    if (gain <= 0) {
      return;
    }
    for (const cue of cues) {
      const sound: number | undefined = this.ready.get(KeySoundPolicy.cueKey(cue.file, cue.semitone));
      if (sound === undefined) {
        continue;
      }
      const parameters: media.PlayParameters = {
        loop: 0, rate: audio.AudioRendererRate.RENDER_RATE_NORMAL, leftVolume: gain, rightVolume: gain,
        priority: 0
      };
      pool.play(sound, parameters).catch((error: Object): void => {
        this.fail(pool, error);
      });
    }
  }

  private fail(pool: media.SoundPool, error: Object): void {
    if (pool !== this.pool || this.failed) {
      return;
    }
    this.failed = true;
    KeyboardLog.error(`key sounds off until the settings change: ${describeError(error)}`);
    this.release();
  }

  private async load(generation: number): Promise<void> {
    if (generation !== this.generation) {
      return;
    }
    const keys: KeySoundPackFiles | null =
      this.selection.pack === null ? null : this.packFiles(this.selection.pack);
    const melody: KeySoundPackFiles | null =
      this.selection.melodyPack === null ? null : this.packFiles(this.selection.melodyPack);
    if ((this.selection.pack !== null && keys === null)
      || (this.selection.melodyPack !== null && melody === null)) {
      return;
    }
    if (melody !== null && melody.sequence === null) {
      KeyboardLog.error(`sound pack ${melody.id}: not a melody`);
      return;
    }
    const pool: media.SoundPool = await media.createSoundPool(POOL_STREAMS, {
      usage: audio.StreamUsage.STREAM_USAGE_MUSIC, rendererFlags: 0
    });
    if (generation !== this.generation) {
      pool.release().catch((error: Object): void => {
        KeyboardLog.error(`key sound pool would not release: ${describeError(error)}`);
      });
      return;
    }
    this.pool = pool;
    this.keys = keys;
    this.melody = melody;
    pool.on('loadComplete', (sound: number): void => {
      if (generation === this.generation) {
        this.completed.add(sound);
        this.settle(sound);
      }
    });
    // A sample the pool cannot decode is reported here; it costs that one sound, not the others. A play that fails is what turns the sounds off.
    pool.on('error', (error: Object): void => {
      if (generation === this.generation) {
        KeyboardLog.error(`key sound pool error: ${describeError(error)}`);
      }
    });
    const requests: KeySoundSampleRequest[] = KeySoundPolicy.samples(keys, melody);
    const bound: KeySoundPackFiles | null = keys !== null ? keys : melody;
    const maxMillis: number = bound === null ? 0 : bound.max_sample_millis;
    for (let index: number = 0; index < requests.length; index++) {
      const directory: string = `${this.cacheRoot}/${generation}/${index}`;
      fs.mkdirSync(directory, true);
      await this.prepare(generation, pool, requests[index], directory, maxMillis);
      if (generation !== this.generation) {
        return;
      }
    }
  }

  /** One pack's validated files, or null after saying why there are none. */
  private packFiles(pack: string): KeySoundPackFiles | null {
    const request: KeySoundPackRequest = {
      state_root: this.stateRoot, sound_packs: this.soundPacks, pack: pack
    };
    const response: KeySoundPackResponse =
      JSON.parse(client.keySoundPack(JSON.stringify(request))) as KeySoundPackResponse;
    if (!response.ok) {
      KeyboardLog.error(`sound pack unavailable: ${response.error}`);
      return null;
    }
    return response.value;
  }

  /** Render or check one sample and hand it to SoundPool. A sample that will not prepare leaves that one sound silent and says why. */
  private async prepare(generation: number, pool: media.SoundPool, request: KeySoundSampleRequest,
    directory: string, maxMillis: number): Promise<void> {
    try {
      if (KeySoundPolicy.isWav(request.file)) {
        const notes: string[] =
          await client.keySoundRenderNotes(request.file, request.semitones, directory, maxMillis);
        for (let index: number = 0; index < notes.length && generation === this.generation; index++) {
          await this.loadFile(generation, pool, notes[index],
            [KeySoundPolicy.cueKey(request.file, request.semitones[index])]);
        }
        return;
      }
      if (!(await this.lengthAllowed(request.file, maxMillis))) {
        KeyboardLog.error('sound pack sample refused: longer than the pack bound or of unknown length');
        return;
      }
      if (generation === this.generation) {
        await this.loadFile(generation, pool, request.file, request.semitones.map((semitone: number): string =>
          KeySoundPolicy.cueKey(request.file, semitone)));
      }
    } catch (error) {
      KeyboardLog.error(`sound pack sample unavailable: ${describeError(error)}`);
    }
  }

  private async loadFile(generation: number, pool: media.SoundPool, path: string,
    cues: string[]): Promise<void> {
    const file: fs.File = fs.openSync(path, fs.OpenMode.READ_ONLY);
    let sound: number = 0;
    let loaded: boolean = false;
    try {
      sound = await pool.load(file.fd, 0, fs.statSync(file.fd).size);
      loaded = true;
    } finally {
      if (!loaded) {
        fs.closeSync(file);
      }
    }
    if (generation !== this.generation) {
      fs.closeSync(file);
      return;
    }
    this.loading.set(sound, file);
    this.loadingCues.set(sound, cues);
    this.settle(sound);
  }

  /** Publish a sound once SoundPool has both named it and finished loading it, and let its descriptor go. */
  private settle(sound: number): void {
    const cues: string[] | undefined = this.loadingCues.get(sound);
    const file: fs.File | undefined = this.loading.get(sound);
    if (cues === undefined || file === undefined || !this.completed.has(sound)) {
      return;
    }
    for (const cue of cues) {
      this.ready.set(cue, sound);
    }
    this.loadingCues.delete(sound);
    this.loading.delete(sound);
    this.completed.delete(sound);
    fs.closeSync(file);
  }

  /** Whether the media service reads the sample as no longer than the pack bound, before SoundPool decodes it. */
  private async lengthAllowed(path: string, maxMillis: number): Promise<boolean> {
    const file: fs.File = fs.openSync(path, fs.OpenMode.READ_ONLY);
    try {
      const extractor: media.AVMetadataExtractor = await media.createAVMetadataExtractor();
      try {
        extractor.fdSrc = { fd: file.fd, offset: 0, length: fs.statSync(file.fd).size };
        const metadata: media.AVMetadata = await extractor.fetchMetadata();
        return KeySoundPolicy.durationAllowed(metadata.duration, maxMillis);
      } finally {
        await extractor.release();
      }
    } finally {
      fs.closeSync(file);
    }
  }

  /** Rendered notes belong to one load; the next load, or none, starts from an empty directory. */
  private clearNotes(): void {
    if (this.cacheRoot.length === 0) {
      return;
    }
    try {
      if (fs.accessSync(this.cacheRoot)) {
        fs.rmdirSync(this.cacheRoot);
      }
    } catch (error) {
      KeyboardLog.error(`rendered key sounds not removed: ${describeError(error)}`);
    }
  }
}

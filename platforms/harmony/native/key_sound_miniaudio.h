#pragma once

// The part of miniaudio the key-sound renderer uses: the WAV decoder with its sample-rate converter, and the WAV encoder. Playback is SoundPool's, so no device backend, engine or other codec is compiled in; every translation unit that includes miniaudio.h does so through here, so the implementation and the declarations agree on what exists.
#define MA_NO_DEVICE_IO
#define MA_NO_RUNTIME_LINKING
#define MA_NO_ENGINE
#define MA_NO_NODE_GRAPH
#define MA_NO_RESOURCE_MANAGER
#define MA_NO_GENERATION
#define MA_NO_MP3
#define MA_NO_FLAC

#include "miniaudio.h"

// The one translation unit that compiles miniaudio's implementation, at the toolchain's default warning level like the Windows host's copy (platforms/windows/CMakeLists.txt, msime-windows-audio); key_sound_render.cpp uses it through its declarations.
#define MINIAUDIO_IMPLEMENTATION
#include "key_sound_miniaudio.h"

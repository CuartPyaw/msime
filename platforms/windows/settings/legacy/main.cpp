#include "SettingsWindow.h"
#include "../../compat/WindowsDpi.h"

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int show) {
  msime::windows::compat::process_context(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
  msime::settings::legacy::SettingsWindow window;
  if (!window.create(instance)) return 1;
  return window.run(show);
}

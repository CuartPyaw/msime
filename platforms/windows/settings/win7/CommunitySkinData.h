#pragma once

#include "ThemeData.h"
#include <wincrypt.h>
#include <cstring>

namespace msime::settings::win7::community_data {

inline theme_data::Image preview(const Json& response) {
  theme_data::Image image;
  const auto value = theme_data::value(response);
  if (!value || !value->is_object() || !value->contains("dataUrl") || !value->at("dataUrl").is_string()) return image;
  const auto url = value->at("dataUrl").get<std::string>();
  const auto comma = url.find(',');
  if (comma == std::string::npos || (url.substr(0, comma) != "data:image/png;base64" && url.substr(0, comma) != "data:image/jpeg;base64")) return image;
  // 社区预览不超过 256 KiB；先限制编码体，再分配与解码。
  const auto encoded = url.substr(comma + 1);
  if (encoded.empty() || encoded.size() > 349528) return image;
  DWORD length = 0;
  if (!CryptStringToBinaryA(encoded.data(), static_cast<DWORD>(encoded.size()), CRYPT_STRING_BASE64 | CRYPT_STRING_STRICT,
      nullptr, &length, nullptr, nullptr) || !length || length > 256 * 1024) return image;
  std::vector<uint8_t> bytes(length);
  if (!CryptStringToBinaryA(encoded.data(), static_cast<DWORD>(encoded.size()), CRYPT_STRING_BASE64 | CRYPT_STRING_STRICT,
      bytes.data(), &length, nullptr, nullptr)) return image;
  const bool png = comma == 21;
  if ((png && (length < 8 || std::memcmp(bytes.data(), "\x89PNG\r\n\x1a\n", 8))) ||
      (!png && (length < 3 || bytes[0] != 0xff || bytes[1] != 0xd8 || bytes[2] != 0xff))) return image;
  if (msime_client_skin_image_decode(bytes.data(), length, theme_data::Image::accept, &image) != 1) return {};
  return image;
}

inline bool draw(const DRAWITEMSTRUCT& draw, const theme_data::Image& image) {
  const auto dc = draw.hDC;
  FillRect(dc, &draw.rcItem, GetSysColorBrush(COLOR_WINDOW));
  if (image.bgra.empty()) {
    const auto saved = SaveDC(dc);
    SetBkMode(dc, TRANSPARENT); SetTextColor(dc, GetSysColor(COLOR_WINDOWTEXT));
    auto rect = draw.rcItem;
    DrawTextW(dc, L"选择皮肤后读取预览图", -1, &rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    RestoreDC(dc, saved); return true;
  }
  BITMAPINFO info{};
  info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  info.bmiHeader.biWidth = static_cast<LONG>(image.width);
  info.bmiHeader.biHeight = -static_cast<LONG>(image.height);
  info.bmiHeader.biPlanes = 1; info.bmiHeader.biBitCount = 32; info.bmiHeader.biCompression = BI_RGB;
  void* bits = nullptr;
  const auto bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &bits, nullptr, 0);
  if (!bitmap || !bits) { if (bitmap) DeleteObject(bitmap); return true; }
  std::memcpy(bits, image.bgra.data(), image.bgra.size());
  const auto memory = CreateCompatibleDC(dc);
  if (!memory) { DeleteObject(bitmap); return true; }
  const auto previous = SelectObject(memory, bitmap);
  const int available_width = draw.rcItem.right - draw.rcItem.left, available_height = draw.rcItem.bottom - draw.rcItem.top;
  const double scale = std::min(static_cast<double>(available_width) / image.width, static_cast<double>(available_height) / image.height);
  if (scale > 0) {
    const int width = std::max(1, static_cast<int>(image.width * scale)), height = std::max(1, static_cast<int>(image.height * scale));
    const BLENDFUNCTION blend{AC_SRC_OVER, 0, 255, AC_SRC_ALPHA};
    AlphaBlend(dc, draw.rcItem.left + (available_width - width) / 2, draw.rcItem.top + (available_height - height) / 2,
        width, height, memory, 0, 0, static_cast<int>(image.width), static_cast<int>(image.height), blend);
  }
  SelectObject(memory, previous); DeleteDC(memory); DeleteObject(bitmap);
  return true;
}
} // namespace msime::settings::win7::community_data

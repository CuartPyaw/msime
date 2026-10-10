#include "../../src/candidate/ToolbarCoordinates.h"
#include <cassert>
#include <limits>
using namespace msime::windows;
int main() {
  assert(toolbar_pixel_unit(120, 1.0) == 1.25);
  assert(toolbar_pixel_unit(144, 1.0) == 1.5);
  assert(toolbar_pixel_unit(0, 1.0) == 1.0);
  for (unsigned dpi : {96u, 120u, 144u, 168u, 192u})
    for (double scale : {0.75, 1.0, 1.25, 1.5})
      for (double font : {16.0, 24.0, 28.0}) {
        const auto metrics = toolbar_metrics(font);
        const auto unit = toolbar_pixel_unit(dpi, scale);
        for (size_t count : {5u, 11u}) {
          for (size_t i = 0; i < count; ++i) {
            const auto box = toolbar_cell(i, metrics);
            for (double x : {box.left + 0.01, (box.left + box.right) / 2, box.right - 0.01}) {
              const double y = (box.top + box.bottom) / 2 * unit;
              assert(toolbar_button_at_pixel(x * unit, y, dpi, scale, count, metrics) == i);
              assert(!toolbar_drag_at_pixel(x * unit, y, dpi, scale, metrics));
              assert(!toolbar_button_at_pixel(x * unit, 0, dpi, scale, count, metrics));
              assert(!toolbar_button_at_pixel(x * unit, (metrics.shadow.top + metrics.height) * unit,
                                              dpi, scale, count, metrics));
            }
          }
          const double x = (metrics.shadow.left + metrics.handle / 2) * unit;
          const double y = (metrics.shadow.top + metrics.height / 2) * unit;
          assert(toolbar_drag_at_pixel(x, y, dpi, scale, metrics));
          assert(!toolbar_drag_at_pixel(x, 0, dpi, scale, metrics));
          assert(!toolbar_button_at_pixel(x, y, dpi, scale, count, metrics));
        }
      }
  assert(!toolbar_button_at_pixel(std::numeric_limits<double>::infinity(), 30, 96, 1, 11, toolbar_metrics(24)));
  // 右键菜单只在卡片上打开：logo、拖动条和按钮都算，四周的阴影边距和最后一个按钮之后不算。
  for (unsigned dpi : {96u, 144u})
    for (double scale : {1.0, 1.5})
      for (bool logo : {true, false}) {
        const auto metrics = toolbar_metrics(24.0, true, logo);
        const auto unit = toolbar_pixel_unit(dpi, scale);
        const auto card = toolbar_card(7, metrics);
        const double y = (card.top + card.bottom) / 2 * unit;
        assert(toolbar_card_at_pixel((card.left + 1) * unit, y, dpi, scale, 7, metrics));
        assert(toolbar_card_at_pixel((card.right - 1) * unit, y, dpi, scale, 7, metrics));
        assert(!toolbar_card_at_pixel((card.left - 1) * unit, y, dpi, scale, 7, metrics));
        assert(!toolbar_card_at_pixel((card.right + 1) * unit, y, dpi, scale, 7, metrics));
        assert(!toolbar_card_at_pixel((card.left + 1) * unit, (card.top - 1) * unit, dpi, scale, 7, metrics));
        assert(!toolbar_card_at_pixel((card.left + 1) * unit, card.bottom * unit, dpi, scale, 7, metrics));
      }
  assert(!toolbar_card_at_pixel(std::numeric_limits<double>::quiet_NaN(), 30, 96, 1, 7, toolbar_metrics(24)));
}

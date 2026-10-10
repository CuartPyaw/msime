import UIKit
import XCTest

/// 内存警告不能让候选条和引擎的候选列表对不上，又要在组字结束后补清引擎缓存（#6685）。
@MainActor
final class KeyboardMemoryTests: XCTestCase {
  private var savedInlineStyle: Any?
  private var savedScheme: ChineseInputScheme?

  override func setUp() {
    super.setUp()
    savedInlineStyle = InlinePreeditPreference.defaults.object(forKey: InlinePreeditPreference.styleKey)
    savedScheme = InputSchemePreference.scheme
  }

  override func tearDown() {
    InlinePreeditPreference.defaults.set(savedInlineStyle, forKey: InlinePreeditPreference.styleKey)
    if let savedScheme { InputSchemePreference.scheme = savedScheme }
    super.tearDown()
  }

  private func descendants(_ view: UIView) -> [UIView] {
    [view] + view.subviews.flatMap { descendants($0) }
  }

  private func press(_ label: String, in controller: KeyboardViewController) throws {
    let key = try XCTUnwrap(
      descendants(controller.view).first { $0.accessibilityLabel == label } as? UIButton, "no key labelled \(label)")
    key.sendActions(for: .primaryActionTriggered)
  }

  private func preedit(_ controller: KeyboardViewController) throws -> String? {
    let button = try XCTUnwrap(
      descendants(controller.view).first { $0.accessibilityIdentifier == "preeditButton" } as? UIButton)
    return button.configuration?.title
  }

  private func composingKeyboard() -> KeyboardViewController {
    enableAllInputSchemes()
    InlinePreeditPreference.style = .off
    InputSchemePreference.scheme = .quanpin
    let controller = KeyboardViewController()
    controller.loadViewIfNeeded()
    controller.view.frame = CGRect(x: 0, y: 0, width: 390, height: KeyboardViewController.defaultKeyboardHeight)
    controller.viewWillAppear(false)
    controller.view.layoutIfNeeded()
    return controller
  }

  /// 候选条上可见的词块，按序号排列。
  private func shownCandidates(_ controller: KeyboardViewController) -> [String] {
    descendants(controller.view)
      .compactMap { view -> (Int, String)? in
        guard let button = view as? UIButton, !button.isHidden,
              let identifier = button.accessibilityIdentifier, identifier.hasPrefix("candidate-"),
              let index = Int(identifier.dropFirst("candidate-".count)),
              let title = button.configuration?.title, !title.isEmpty else { return nil }
        return (index, title)
      }
      .sorted { $0.0 < $1.0 }
      .map(\.1)
  }

  // MARK: - 内存警告与组字

  /// 点候选条上的第 `number` 个词块（从 1 数起）。
  private func tapCandidate(_ number: Int, in controller: KeyboardViewController) throws {
    let chip = try XCTUnwrap(
      descendants(controller.view).first { $0.accessibilityIdentifier == "candidate-\(number)" && !$0.isHidden } as? UIButton,
      "no candidate-\(number) on the strip")
    chip.sendActions(for: .primaryActionTriggered)
  }

  /// 组字中收到内存警告：引擎缓存不动，组字和候选条原样保留；选词上屏结束这次组字（推迟的清缓存在这时补上）后接着打字照常。这一条在组字中也清缓存的旧写法上同样通过，它守的是组字不被打断。
  func testAMemoryWarningMidCompositionKeepsTheComposition() throws {
    let controller = composingKeyboard()
    for letter in ["N", "I", "H", "A", "O"] { try press("字母 \(letter)", in: controller) }
    controller.view.layoutIfNeeded()
    let composing = try preedit(controller)
    let candidates = shownCandidates(controller)
    XCTAssertFalse(candidates.isEmpty)

    controller.didReceiveMemoryWarning()
    controller.view.layoutIfNeeded()
    XCTAssertEqual(try preedit(controller), composing)
    XCTAssertEqual(shownCandidates(controller), candidates)

    try tapCandidate(1, in: controller)
    controller.view.layoutIfNeeded()
    XCTAssertEqual(shownCandidates(controller), [], "selecting the whole word should end the composition")
    for letter in ["N", "I"] { try press("字母 \(letter)", in: controller) }
    controller.view.layoutIfNeeded()
    XCTAssertNotEqual(try preedit(controller), composing)
    XCTAssertFalse(shownCandidates(controller).isEmpty)
    controller.viewWillDisappear(false)
  }

  /// 没有组字时内存警告照常清引擎缓存，键盘随后正常组字。
  func testAMemoryWarningWhileIdleLeavesTheKeyboardUsable() throws {
    let controller = composingKeyboard()
    let idle = try preedit(controller)

    controller.didReceiveMemoryWarning()
    for letter in ["N", "I"] { try press("字母 \(letter)", in: controller) }
    controller.view.layoutIfNeeded()
    XCTAssertNotEqual(try preedit(controller), idle)
    XCTAssertFalse(shownCandidates(controller).isEmpty)
    controller.viewWillDisappear(false)
  }
}

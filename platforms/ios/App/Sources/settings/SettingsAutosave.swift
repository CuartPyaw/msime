import SwiftUI

/// Automatic saving for a native settings page, the same behaviour as the shared React settings (`use-settings-persistence.ts`): typing is saved once it pauses, pickers and toggles are saved at once, anything still waiting is saved when the page goes away or the app leaves the foreground, and a failed save stays on screen with 重试 instead of being lost.
@MainActor
final class SettingsAutosave: ObservableObject {
  enum Phase: Equatable {
    case idle
    /// A save is waiting for typing to pause, or running.
    case saving
    case saved
    /// The value on screen cannot be saved as it is (a half-typed address, say); the message says why. Nothing is written until the value changes.
    case invalid(String)
    case failed(String)
  }

  /// The pause after the last edit before it is saved, as `SETTINGS_AUTOSAVE_DELAY_MS` in the shared settings.
  nonisolated static let defaultDelay: Duration = .milliseconds(400)
  /// How long 已保存 stays before the status clears, as `SETTINGS_SAVED_STATUS_MS`.
  nonisolated static let savedStatusDuration: Duration = .seconds(2)

  @Published private(set) var phase = Phase.idle
  private let delay: Duration
  private let savedStatusDuration: Duration
  private var pending: (() throws -> Void)?
  private var countdown: Task<Void, Never>?
  private var savedStatus: Task<Void, Never>?
  private var failedSave: (() throws -> Void)?

  init(delay: Duration = SettingsAutosave.defaultDelay, savedStatusDuration: Duration = SettingsAutosave.savedStatusDuration) {
    self.delay = delay
    self.savedStatusDuration = savedStatusDuration
  }

  /// Whether a save is waiting for typing to pause.
  var hasPending: Bool { pending != nil }

  /// Runs `save` once edits have paused for the delay. Each call restarts the countdown and replaces the save waiting to run, so only the latest value is written.
  func schedule(_ save: @escaping () throws -> Void) {
    pending = save
    setPhase(.saving)
    countdown?.cancel()
    countdown = Task { [weak self, delay] in
      try? await Task.sleep(for: delay)
      guard !Task.isCancelled else { return }
      self?.flush()
    }
  }

  /// Runs `save` at once, replacing any save still waiting for its pause.
  func saveNow(_ save: @escaping () throws -> Void) {
    dropPending()
    run(save)
  }

  /// Runs the save waiting for its pause now, if there is one.
  func flush() {
    guard let save = pending else { return }
    dropPending()
    run(save)
  }

  /// Shows why the value on screen is not saved and drops the save waiting for it, so a half-typed value never reaches storage.
  func reject(_ message: String) {
    dropPending()
    failedSave = nil
    setPhase(.invalid(message))
  }

  /// Drops a waiting save and any status, for a page that has just reloaded what is stored.
  func reset() {
    dropPending()
    failedSave = nil
    setPhase(.idle)
  }

  /// Runs the save that failed again.
  func retry() {
    guard case .failed = phase, let save = failedSave else { return }
    run(save)
  }

  private func dropPending() {
    pending = nil
    countdown?.cancel()
    countdown = nil
  }

  private func run(_ save: @escaping () throws -> Void) {
    do {
      try save()
      failedSave = nil
      setPhase(.saved)
      savedStatus = Task { [weak self, savedStatusDuration] in
        try? await Task.sleep(for: savedStatusDuration)
        guard !Task.isCancelled, let self, self.phase == .saved else { return }
        self.phase = .idle
      }
    } catch {
      failedSave = save
      setPhase(.failed(error.localizedDescription))
    }
  }

  private func setPhase(_ next: Phase) {
    savedStatus?.cancel()
    savedStatus = nil
    phase = next
  }
}

/// The save status of an autosaving page: 正在保存… / 已保存, why the value is not saved, or the failure with 重试.
struct SettingsAutosaveStatus: View {
  @ObservedObject var autosave: SettingsAutosave

  var body: some View {
    switch autosave.phase {
    case .idle:
      EmptyView()
    case .saving:
      Text("正在保存…").foregroundStyle(.secondary).accessibilityIdentifier("settingsAutosaveStatus")
    case .saved:
      Text("已保存").foregroundStyle(.secondary).accessibilityIdentifier("settingsAutosaveStatus")
    case .invalid(let message):
      Text(message).foregroundStyle(.red).accessibilityIdentifier("settingsAutosaveStatus")
    case .failed(let message):
      HStack {
        Text(message).foregroundStyle(.red).accessibilityIdentifier("settingsAutosaveStatus")
        Spacer()
        Button("重试") { autosave.retry() }.accessibilityIdentifier("settingsAutosaveRetry")
      }
    }
  }
}

extension View {
  /// Saves whatever is still waiting for its pause when the page goes away or the app leaves the foreground.
  func flushesAutosave(_ autosave: SettingsAutosave) -> some View {
    modifier(SettingsAutosaveFlush(autosave: autosave))
  }
}

private struct SettingsAutosaveFlush: ViewModifier {
  let autosave: SettingsAutosave
  @Environment(\.scenePhase) private var scenePhase

  func body(content: Content) -> some View {
    content
      .onDisappear { autosave.flush() }
      .onChange(of: scenePhase) { _, phase in
        if phase != .active { autosave.flush() }
      }
  }
}

import XCTest

private struct SaveFailure: LocalizedError {
  var errorDescription: String? { "磁盘已满。" }
}

@MainActor
final class SettingsAutosaveTests: XCTestCase {
  func testTypingIsSavedOnceAfterThePauseWithTheLatestValue() async throws {
    let autosave = SettingsAutosave(delay: .milliseconds(50))
    var written: [String] = []
    for value in ["h", "ht", "https"] {
      autosave.schedule { written.append(value) }
    }
    XCTAssertEqual(written, [], "nothing is written while typing continues")
    XCTAssertEqual(autosave.phase, .saving)
    XCTAssertTrue(autosave.hasPending)

    try await waitUntil { !written.isEmpty }
    XCTAssertEqual(written, ["https"], "the edits coalesce into one save of the last value")
    XCTAssertEqual(autosave.phase, .saved)
    XCTAssertFalse(autosave.hasPending)
  }

  func testFlushWritesTheWaitingSaveAtOnceAndOnlyOnce() async throws {
    let autosave = SettingsAutosave(delay: .seconds(60))
    var written: [String] = []
    autosave.schedule { written.append("draft") }
    autosave.flush()
    XCTAssertEqual(written, ["draft"], "leaving the page does not wait out the pause")
    XCTAssertEqual(autosave.phase, .saved)
    autosave.flush()
    XCTAssertEqual(written, ["draft"], "a second flush has nothing left to write")
  }

  func testSaveNowReplacesTheWaitingSave() {
    let autosave = SettingsAutosave(delay: .seconds(60))
    var written: [String] = []
    autosave.schedule { written.append("typed") }
    autosave.saveNow { written.append("picked") }
    autosave.flush()
    XCTAssertEqual(written, ["picked"])
  }

  func testAFailureIsShownAndRetryWritesTheSameValue() {
    let autosave = SettingsAutosave(delay: .seconds(60))
    var fails = true
    var written: [String] = []
    autosave.schedule {
      if fails { throw SaveFailure() }
      written.append("value")
    }
    autosave.flush()
    XCTAssertEqual(autosave.phase, .failed("磁盘已满。"))
    XCTAssertFalse(autosave.hasPending)

    fails = false
    autosave.retry()
    XCTAssertEqual(written, ["value"])
    XCTAssertEqual(autosave.phase, .saved)
    autosave.retry()
    XCTAssertEqual(written, ["value"], "retry does nothing once the save went through")
  }

  func testAnInvalidValueDropsTheWaitingSave() {
    let autosave = SettingsAutosave(delay: .seconds(60))
    var written: [String] = []
    autosave.schedule { written.append("https://valid") }
    autosave.reject("请填写完整的 HTTPS 接口地址和模型名称。")
    autosave.flush()
    XCTAssertEqual(written, [], "the half-typed value never reaches storage")
    XCTAssertEqual(autosave.phase, .invalid("请填写完整的 HTTPS 接口地址和模型名称。"))
    autosave.retry()
    XCTAssertEqual(written, [], "an invalid value has nothing to retry")
  }

  func testSavedStatusClearsAfterItsDuration() async throws {
    let autosave = SettingsAutosave(delay: .seconds(60), savedStatusDuration: .milliseconds(50))
    autosave.saveNow {}
    XCTAssertEqual(autosave.phase, .saved)
    try await waitUntil { autosave.phase == .idle }
  }

  private func waitUntil(_ condition: @MainActor () -> Bool, file: StaticString = #filePath, line: UInt = #line) async throws {
    let deadline = ContinuousClock.now + .seconds(5)
    while !condition() {
      guard ContinuousClock.now < deadline else { return XCTFail("condition not met in time", file: file, line: line) }
      try await Task.sleep(for: .milliseconds(10))
    }
  }
}

import Foundation

// Candidate glosses use the account's bounded translation endpoint. The native input method
// already loads this Swift backend as a private dylib, so the Objective-C++ controller only needs a
// C entry point and a notification carrying display-safe strings back to the main thread.
private enum BackendCandidateGloss {
  static let notification = Notification.Name("MSIMEBackendCandidateTranslationsDidArrive")
  private static let account = BackendAccountSession.shared
  private static let anonymous = BackendAnonymousAccount.session
  private static let client = BackendAccountClient()

  private static func token() async throws -> String {
    if let value = try? await account.accessToken() { return value }
    if let value = try? await anonymous.accessToken() { return value }
    _ = try await BackendAnonymousAccount.ensureSignedIn(session: anonymous, client: client)
    return try await anonymous.accessToken()
  }

  static func fetch(words: [String], primary: String, secondary: String, generation: UInt64) {
    guard !words.isEmpty, !primary.isEmpty else { return }
    Task {
      guard let token = try? await token() else { return }
      await withTaskGroup(of: Void.self) { group in
        for code in [primary, secondary] where !code.isEmpty {
          group.addTask {
            guard let values = try? await client.translate(texts: words, target: code, token: token) else { return }
            // Every word asked about gets an answer, so the input method can remember the ones the account had nothing for and stop asking about them for a while, as Windows does. An empty or unchanged value is sent as "", and a duplicate candidate keeps whichever of its answers is not empty. A request that failed (offline, rate limited) posts nothing, so the words stay unknown and are asked about again.
            let table = Dictionary(zip(words, values).map { ($0.0, $0.1 == $0.0 ? "" : $0.1) },
                                   uniquingKeysWith: { first, second in first.isEmpty ? second : first })
            await MainActor.run {
              NotificationCenter.default.post(name: notification, object: nil, userInfo: [
                // Each reply names its own language, so the input method files it under that target without depending on the order in which the two requests finish, and saves only the English one to the learned glossary.
                "generation": generation,
                "target": code,
                "translations": table,
              ])
            }
          }
        }
      }
    }
  }
}

@_cdecl("MSIMEFetchAccountCandidateGlosses")
public func msimeFetchAccountCandidateGlosses(_ wordsJSON: UnsafePointer<CChar>, _ primary: UnsafePointer<CChar>,
                                               _ secondary: UnsafePointer<CChar>, _ generation: UInt64) {
  guard let data = String(cString: wordsJSON).data(using: .utf8),
        let words = try? JSONDecoder().decode([String].self, from: data),
        words.count <= 32 else { return }
  BackendCandidateGloss.fetch(words: words, primary: String(cString: primary),
                              secondary: String(cString: secondary), generation: generation)
}

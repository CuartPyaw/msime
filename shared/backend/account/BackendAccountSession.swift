import Foundation
import Security

protocol BackendSessionAPI: Sendable {
  func login(challenge: String, credential: String, linkToken: String?) async throws -> BackendAccountClient.Tokens
  func refresh(_ token: String) async throws -> BackendAccountClient.Tokens
  func logout(token: String, all: Bool) async throws
}
extension BackendAccountClient: BackendSessionAPI {}

struct BackendSavedSession: Codable, Sendable {
  let tokens: BackendAccountClient.Tokens
  let expiresAt: Date

  static func validated(_ session: BackendSavedSession, now: Date = Date()) throws -> BackendSavedSession {
    try BackendAccountClient.validate(session.tokens)
    guard session.expiresAt.timeIntervalSinceReferenceDate.isFinite,
          session.expiresAt <= now.addingTimeInterval(TimeInterval(BackendAccountClient.maxSessionSeconds)) else {
      throw BackendAccountClient.Failure(status: 0)
    }
    return session
  }

  static func forTokens(_ tokens: BackendAccountClient.Tokens, now: Date = Date()) throws -> BackendSavedSession {
    try BackendAccountClient.validate(tokens)
    return try validated(.init(tokens: tokens,
      expiresAt: now.addingTimeInterval(TimeInterval(tokens.expires_in))), now: now)
  }
}
protocol BackendSessionStorage: Sendable {
  func load() throws -> BackendSavedSession?
  func save(_ session: BackendSavedSession) throws
  func clear() throws
}
struct BackendKeychain: BackendSessionStorage {
  /// On iOS the session lives in the App Group's keychain access group, which the app and the keyboard extension both already hold as an entitlement, so the keyboard can reach the signed-in account (cloud clipboard) without the token ever being written to a file. Other platforms keep the item in the process's default access group, exactly as before.
  #if os(iOS)
  static let defaultAccessGroup: String? = "group.app.msime.ios"
  #else
  static let defaultAccessGroup: String? = nil
  #endif
  private let accessGroup: String?
  private let service: String

  init(accessGroup: String? = BackendKeychain.defaultAccessGroup, service: String = "app.msime.backend.account") {
    self.accessGroup = accessGroup
    self.service = service
  }
  /// Matches the item in every access group the process holds, which is what clearing and the pre-group lookup need.
  private var anyGroupQuery: [String: Any] {
    [kSecClass as String: kSecClassGenericPassword,
     kSecAttrService as String: service,
     kSecAttrAccount as String: "https://api.msime.app"]
  }
  private var query: [String: Any] {
    var query = anyGroupQuery
    if let accessGroup { query[kSecAttrAccessGroup as String] = accessGroup }
    return query
  }
  func load() throws -> BackendSavedSession? {
    var query = query
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    if status == errSecItemNotFound {
      if let moved = try migrateDefaultGroupSession() { return moved }
      return try migrateCommunitySession()
    }
    // A keychain we cannot read is not a session we have. An unsigned simulator build answers
    // -34018 (errSecMissingEntitlement) here, and a device can answer errSecInteractionNotAllowed
    // while locked; treating either as a hard failure took every screen that asks "am I signed in"
    // down with it, and the only thing the user saw was the status-0 fallback, 请求未完成. Absent
    // and unreadable are the same answer to that question. Data we *can* read but cannot decode is
    // a real fault and still throws.
    guard status == errSecSuccess, let data = result as? Data else { return nil }
    do { return try BackendSavedSession.validated(JSONDecoder().decode(BackendSavedSession.self, from: data)) }
    catch { throw BackendAccountClient.Failure(status: 0) }
  }
  /// Sessions saved before the App Group access group was used sit in the app's default access group, where the keyboard cannot see them. The app moves such an item into the shared group the first time it reads it; the keyboard finds nothing here and reports signed out until then.
  private func migrateDefaultGroupSession() throws -> BackendSavedSession? {
    guard accessGroup != nil else { return nil }
    var lookup = anyGroupQuery
    lookup[kSecReturnData as String] = true
    lookup[kSecReturnAttributes as String] = true
    lookup[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(lookup as CFDictionary, &result)
    // Absent or unreadable: nothing to move, for the same reason `load` treats both as signed out.
    guard status == errSecSuccess, let found = result as? [String: Any],
          let data = found[kSecValueData as String] as? Data,
          let group = found[kSecAttrAccessGroup as String] as? String else { return nil }
    let session: BackendSavedSession
    do { session = try BackendSavedSession.validated(JSONDecoder().decode(BackendSavedSession.self, from: data)) }
    catch { throw BackendAccountClient.Failure(status: 0) }
    try save(session)
    // Delete by the old item's own group: a query without one would take the copy just saved with it.
    var old = anyGroupQuery
    old[kSecAttrAccessGroup as String] = group
    if group != accessGroup { SecItemDelete(old as CFDictionary) }
    return session
  }
  private func migrateCommunitySession() throws -> BackendSavedSession? {
    let legacy: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: "app.msime.ios.community", kSecAttrAccount as String: "api.msime.app"]
    var lookup = legacy
    lookup[kSecReturnData as String] = true
    var result: CFTypeRef?
    let status = SecItemCopyMatching(lookup as CFDictionary, &result)
    if status == errSecItemNotFound { return nil }
    // Same reasoning as above: an unreadable legacy item is nothing to migrate, not an error.
    guard status == errSecSuccess, let data = result as? Data else { return nil }
    struct Legacy: Decodable {
      struct User: Decodable { let id: String; let display_name: String; let created_at: String? }
      let access_token: String; let refresh_token: String; let user: User
      let saved_at: Date?; let expires_in: Int?
    }
    let old = try JSONDecoder().decode(Legacy.self, from: data)
    let seconds = old.expires_in ?? 900
    let tokens = BackendAccountClient.Tokens(access_token: old.access_token, refresh_token: old.refresh_token,
      token_type: "Bearer", expires_in: seconds,
      user: .init(id: old.user.id, display_name: old.user.display_name, created_at: old.user.created_at ?? ""))
    let value = try BackendSavedSession.validated(.init(tokens: tokens,
      expiresAt: (old.saved_at ?? .distantPast).addingTimeInterval(TimeInterval(seconds))))
    try save(value)
    SecItemDelete(legacy as CFDictionary)
    return value
  }
  func save(_ session: BackendSavedSession) throws {
    let validatedSession = try BackendSavedSession.validated(session)
    let data = try JSONEncoder().encode(validatedSession)
    let attributes: [String: Any] = [kSecValueData as String: data,
      kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
    var status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
    if status == errSecItemNotFound {
      status = SecItemAdd(query.merging(attributes) { _, new in new } as CFDictionary, nil)
    }
    guard status == errSecSuccess else { throw BackendAccountClient.Failure(status: 0) }
    try clearLegacy()
  }
  private func clearLegacy() throws {
    let status = SecItemDelete([kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: "app.msime.ios.community", kSecAttrAccount as String: "api.msime.app"] as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else { throw BackendAccountClient.Failure(status: 0) }
  }
  func clear() throws {
    try clearLegacy()
    // Every group: a sign-out must also remove a copy that predates the shared access group, or the next read would move it back.
    let status = SecItemDelete(anyGroupQuery as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else { throw BackendAccountClient.Failure(status: 0) }
  }
}

/// Serializes token refreshes across processes that share one stored session. The server rotates the refresh token on every refresh and revokes the whole session when a used one is presented again, so two processes refreshing from the same stored token sign the user out; whoever holds this lock re-reads the store before refreshing.
protocol BackendRefreshLock: Sendable {
  func run<T: Sendable>(_ body: @Sendable () async throws -> T) async throws -> T
}

/// For a session no other process shares.
struct BackendProcessRefreshLock: BackendRefreshLock {
  func run<T: Sendable>(_ body: @Sendable () async throws -> T) async throws -> T { try await body() }
}

/// An exclusive `flock` on a file in a directory every sharing process can open. It is waited for by polling, so a cooperative thread is never blocked, and it is held only for the one refresh request: iOS ends a suspended process that keeps a lock in an App Group container.
struct BackendFileRefreshLock: BackendRefreshLock {
  let url: URL?
  var timeout: TimeInterval = 20

  /// iOS: the App Group container, opened by both the app and the keyboard extension.
  static var appGroup: BackendFileRefreshLock {
    BackendFileRefreshLock(url: FileManager.default
      .containerURL(forSecurityApplicationGroupIdentifier: "group.app.msime.ios")?
      .appendingPathComponent("backend-account-refresh.lock", isDirectory: false))
  }

  func run<T: Sendable>(_ body: @Sendable () async throws -> T) async throws -> T {
    // No shared directory means no way to keep another process out, and refreshing anyway risks the revocation this lock exists to prevent.
    guard let url else { throw BackendAccountClient.Failure(status: 0) }
    let descriptor = open(url.path, O_CREAT | O_RDWR | O_NOFOLLOW | O_CLOEXEC, S_IRUSR | S_IWUSR)
    guard descriptor >= 0 else { throw BackendAccountClient.Failure(status: 0) }
    defer { close(descriptor) }
    let deadline = Date().addingTimeInterval(timeout)
    while flock(descriptor, LOCK_EX | LOCK_NB) != 0 {
      guard errno == EWOULDBLOCK || errno == EINTR, Date() < deadline else { throw BackendAccountClient.Failure(status: 0) }
      try await Task.sleep(nanoseconds: 50_000_000)
    }
    defer { flock(descriptor, LOCK_UN) }
    return try await body()
  }
}

/// Serializes rotating refresh tokens. A late login/refresh may never undo logout.
actor BackendAccountSession {
  static let shared = BackendAccountSession()
  private let api: any BackendSessionAPI
  private let storage: any BackendSessionStorage
  private var saved: BackendSavedSession?
  private var loaded = false
  private var generation = 0
  private var refreshing: Task<String, Error>?
  private let refreshLock: any BackendRefreshLock

  /// On iOS the app and the keyboard extension share the stored sessions, so refreshes take the App Group lock there.
  #if os(iOS)
  static var defaultRefreshLock: any BackendRefreshLock { BackendFileRefreshLock.appGroup }
  #else
  static var defaultRefreshLock: any BackendRefreshLock { BackendProcessRefreshLock() }
  #endif

  init(api: any BackendSessionAPI = BackendAccountClient(), storage: any BackendSessionStorage = BackendKeychain(),
       refreshLock: any BackendRefreshLock = BackendAccountSession.defaultRefreshLock) {
    self.api = api; self.storage = storage; self.refreshLock = refreshLock
  }
  func user() throws -> BackendAccountClient.User? {
    try load()
    return saved?.tokens.user
  }
  private func load() throws {
    if !loaded {
      saved = try storage.load().map { try BackendSavedSession.validated($0) }
      loaded = true
    }
  }
  func signIn(challenge: String, credential: String) async throws {
    generation += 1
    refreshing?.cancel(); refreshing = nil
    let version = generation
    let tokens = try await api.login(challenge: challenge, credential: credential, linkToken: nil)
    try Task.checkCancellation()
    guard version == generation else { throw CancellationError() }
    try install(tokens)
  }
  private func install(_ tokens: BackendAccountClient.Tokens) throws {
    let value = try BackendSavedSession.forTokens(tokens)
    try storage.save(value)
    saved = value; loaded = true
  }
  func accessToken(retrying rejectedToken: String? = nil) async throws -> String {
    try load()
    // A refresh may have been started because another caller received a 401
    // for the still-unexpired access token. Every concurrent caller must join
    // it before returning that rejected token from the fast path.
    if let refreshing { return try await refreshing.value }
    if let current = saved, current.expiresAt.timeIntervalSinceNow > 30 && rejectedToken != current.tokens.access_token {
      return current.tokens.access_token
    }
    // Other actors and processes share this storage and may already have rotated (or created) the session.
    if let stored = try? storage.load().map({ try BackendSavedSession.validated($0) }),
       stored.tokens.refresh_token != saved?.tokens.refresh_token { saved = stored }
    guard let current = saved else { throw BackendAccountClient.Failure(status: 401) }
    if current.expiresAt.timeIntervalSinceNow > 30 && rejectedToken != current.tokens.access_token { return current.tokens.access_token }
    let refreshToken = current.tokens.refresh_token
    let version = generation
    let task = Task<String, Error> {
      try await self.refreshLock.run {
        try await self.refreshHoldingLock(refreshToken, rejectedToken: rejectedToken, version: version)
      }
    }
    refreshing = task
    defer { if version == generation { refreshing = nil } }
    return try await task.value
  }
  /// Runs with the refresh lock held, so no other process can rotate the stored session between the read below and the save after the refresh.
  private func refreshHoldingLock(_ expected: String, rejectedToken: String?, version: Int) async throws -> String {
    var refreshToken = expected
    // Another process may have rotated the session while this one waited for the lock; refreshing from the token it already used would revoke the session.
    if let stored = try? storage.load().map({ try BackendSavedSession.validated($0) }), stored.tokens.refresh_token != expected {
      guard generation == version else { throw CancellationError() }
      saved = stored
      if stored.expiresAt.timeIntervalSinceNow > 30 && rejectedToken != stored.tokens.access_token { return stored.tokens.access_token }
      refreshToken = stored.tokens.refresh_token
    }
    do {
      let tokens = try await api.refresh(refreshToken)
      guard generation == version else { throw CancellationError() }
      try install(tokens)
      return tokens.access_token
    } catch {
      if version == generation, let failure = error as? BackendAccountClient.Failure, failure.status == 401 {
        // Clear only the session that was rejected; a rotation saved meanwhile elsewhere is adopted.
        // A nil read may be an unreadable keychain rather than an absent item, so it is not cleared.
        let stored = try? storage.load().map({ try BackendSavedSession.validated($0) })
        if let stored, stored.tokens.refresh_token != refreshToken {
          saved = stored
          if stored.expiresAt.timeIntervalSinceNow > 30 && rejectedToken != stored.tokens.access_token { return stored.tokens.access_token }
        } else {
          if stored != nil { try storage.clear() }
          saved = nil
        }
      }
      throw error
    }
  }
  func updateUser(_ user: BackendAccountClient.User, matching token: String) throws {
    try load()
    guard let current = saved, current.tokens.access_token == token, current.tokens.user.id == user.id else {
      throw CancellationError()
    }
    let old = current.tokens
    let tokens = BackendAccountClient.Tokens(access_token: old.access_token, refresh_token: old.refresh_token,
      token_type: old.token_type, expires_in: old.expires_in, user: user)
    let value = try BackendSavedSession.validated(.init(tokens: tokens, expiresAt: current.expiresAt))
    try storage.save(value); saved = value
  }
  func credentials(retrying rejectedToken: String? = nil, matchingUserID expected: String? = nil) async throws -> (userID: String, token: String) {
    try load()
    if let expected, saved?.tokens.user.id != expected { throw CancellationError() }
    let token = try await accessToken(retrying: rejectedToken)
    guard let saved, saved.tokens.access_token == token,
          expected == nil || saved.tokens.user.id == expected else { throw CancellationError() }
    return (saved.tokens.user.id, token)
  }
  func forget() throws {
    generation += 1
    refreshing?.cancel(); refreshing = nil
    saved = nil; loaded = true
    try storage.clear()
  }
  func logout(all: Bool = false) async throws {
    let token: String
    do { token = try await accessToken() }
    catch { try forget(); throw error }
    try forget()
    try await api.logout(token: token, all: all)
  }
}

import SwiftUI

/// 系统认证只负责本次页面访问，授权前不创建统计内容。
struct StatisticsAuthenticationGate<Content: View>: View {
  @ViewBuilder let content: () -> Content
  @Environment(\.scenePhase) private var scenePhase
  @StateObject private var authentication = StatisticsAuthentication()

  var body: some View {
    Group {
      if authentication.authorized && scenePhase == .active {
        content()
      } else {
        VStack(spacing: 16) {
          Image(systemName: "lock.shield").font(.largeTitle)
          Text(authentication.message.isEmpty ? "验证身份后查看打字统计" : authentication.message)
          if authentication.busy {
            ProgressView("正在验证…")
          } else {
            Button("验证身份") { authentication.authenticate() }
              .buttonStyle(.borderedProminent)
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .designCanvasBackground()
        .navigationTitle("统计")
      }
    }
    .onAppear { if scenePhase == .active { authentication.authenticate(automatically: true) } }
    .onChange(of: scenePhase) { _, phase in
      if phase == .background { authentication.lock() }
      if phase == .active { authentication.authenticate(automatically: true) }
    }
    .onDisappear { authentication.lock() }
  }
}


import SwiftUI
import UniformTypeIdentifiers

/// 「自定义候选释义」: the Windows `custom_translations.txt`, edited in place or taken from a file picked in Files, and saved where the keyboard's Engine reads it.
struct CustomTranslationsView: View {
  @State private var text = ""
  /// What the file holds, so reading it in or an unchanged edit does not write it again.
  @State private var savedText = ""
  @State private var loaded = false
  @State private var notice = ""
  @State private var importing = false
  @StateObject private var autosave = SettingsAutosave()
  private let url = CustomTranslations.defaultURL

  var body: some View {
    Form {
      Section {
        TextEditor(text: $text)
          .font(.body.monospaced())
          .textInputAutocapitalization(.never).autocorrectionDisabled()
          .frame(minHeight: 220)
          .overlay(alignment: .topLeading) {
            if text.isEmpty {
              Text(CustomTranslations.example).font(.body.monospaced()).foregroundStyle(.tertiary)
                .padding(.top, 8).padding(.leading, 5).allowsHitTesting(false)
            }
          }
          .accessibilityLabel("自定义候选释义").accessibilityIdentifier("customTranslationsText")
          .onChange(of: text) { _, _ in
            notice = ""
            scheduleSave()
          }
      } footer: {
        VStack(alignment: .leading, spacing: 4) {
          Text(notice.isEmpty ? summary : notice).accessibilityIdentifier("customTranslationsStatus")
          SettingsAutosaveStatus(autosave: autosave)
        }
      }
      Section {
        Button("从文件导入…") { importing = true }.accessibilityIdentifier("customTranslationsImport")
      } footer: {
        Text("每行一条，用 Tab 分隔源词和译文；以 # 开头的行是注释。源词含汉字即为中译英，全是英文则为英译中。同一个源词写多次时以最后一次为准。与 Windows 放进用户目录的 custom_translations.txt 格式相同，可以直接导入。键盘下次载入时生效，切到别的键盘再切回来即可。")
      }
    }
    .navigationTitle("自定义候选释义")
    .navigationBarTitleDisplayMode(.inline)
    .onAppear(perform: load)
    .flushesAutosave(autosave)
    .fileImporter(isPresented: $importing, allowedContentTypes: [.plainText, .text]) { result in
      guard case .success(let picked) = result else { return }
      let scoped = picked.startAccessingSecurityScopedResource()
      defer { if scoped { picked.stopAccessingSecurityScopedResource() } }
      do {
        text = try CustomTranslations.read(at: picked)
      } catch { notice = error.localizedDescription }
    }
  }

  private var summary: String {
    guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return "尚未添加释义。" }
    let report = CustomTranslations.parse(text)
    return "\(report.entries) 条释义（中译英 \(report.chineseSources) 条）" + (report.skipped > 0 ? "，\(report.skipped) 行无法识别" : "") + "。"
  }

  private func load() {
    guard !loaded else { return }
    guard let url else { notice = "共享容器不可用。"; return }
    do {
      text = try CustomTranslations.read(at: url)
      savedText = text
      loaded = true
    } catch { notice = error.localizedDescription }
  }

  /// Saves the editor once typing pauses. Nothing is written before the file has been read, so a failed read never overwrites it; an edit typed back to what is saved still replaces the save waiting for the edit before it.
  private func scheduleSave() {
    guard loaded, let url, text != savedText || autosave.hasPending else { return }
    let content = text
    autosave.schedule {
      try CustomTranslations.write(content, to: url)
      savedText = content
    }
  }
}

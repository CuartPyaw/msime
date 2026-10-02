//! 开发和打包用：把按需下载的资源包装到指定的 state_root 下（`<state_root>/resource-packs/<id>/`），与 App 运行时下载的位置和格式一致。不带资源包 id 时安装全部。
use msime_client_core::resource_packs::{self, ResourcePack};
use std::sync::atomic::AtomicBool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let usage = "usage: install_resource_pack <absolute-state-root> [pack-id ...]";
    let mut arguments = std::env::args().skip(1);
    let state_root = std::path::PathBuf::from(arguments.next().ok_or(usage)?);
    let mut packs = Vec::new();
    for id in arguments {
        packs.push(ResourcePack::from_id(&id).ok_or_else(|| format!("unknown pack: {id}"))?);
    }
    if packs.is_empty() {
        packs.extend(ResourcePack::ALL);
    }
    let cancel = AtomicBool::new(false);
    for pack in packs {
        const MIB: u64 = 1024 * 1024;
        let mut reported = 0u64;
        let path = resource_packs::install(
            &state_root,
            pack,
            "",
            &mut |event| {
                // 每 MiB 最多输出一次，阶段结束时再补一行。
                if event.stage != "download" || event.downloaded - reported >= MIB {
                    reported = event.downloaded;
                    eprintln!(
                        "{} {} {}/{}",
                        pack.id(),
                        event.stage,
                        event.downloaded,
                        event.total
                    );
                }
            },
            &cancel,
        )?;
        println!("{}", path.display());
    }
    Ok(())
}

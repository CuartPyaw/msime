#!/usr/bin/env bash
set -euo pipefail
umask 077

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$repo_root"
source_dir=${1:?usage: stage-resources.sh <verified-resource-directory> [offline-glosses-directory]}
source_dir=$(cd "$source_dir" && pwd)
destination="$repo_root/target/ios/EngineResources"
artifacts=$(cargo run --quiet -p msime-client-core --example verify_resources --locked -- "$source_dir")
mkdir -p "$destination"
while IFS= read -r artifact; do
  cp "$source_dir/$artifact" "$destination/$artifact"
done <<< "$artifacts"
cargo run --quiet -p msime-client-core --example verify_resources --locked -- "$destination" >/dev/null
# Helpcode tables are not part of the dictionary release; the repository carries them in resources/helpcodes, and the Engine reads them from helpcodes/ under the resource directory (crates/engine/src/assets.rs names the six files). Without them the Engine has nothing to match: Shift letters are taken as helpcode and narrow nothing.
helpcodes="$repo_root/resources/helpcodes"
rm -rf "$destination/helpcodes"
mkdir -p "$destination/helpcodes"
for table in helpcode.txt zrm_helpcode_big_unique.txt shouyou2_0_helpcode.txt shouyouplus_helpcode.txt xiaohe_helpcode.txt jiajia_helpcode.txt; do
  cp "$helpcodes/$table" "$destination/helpcodes/$table"
done
cp "$helpcodes/ENGINE-NOTICE.md" "$destination/helpcodes/NOTICE.md"
cp "$helpcodes/NOTICE.md" "$destination/helpcodes/NOTICE-jiajia.md"
# Optional: non-English candidate glosses built by scripts/build_offline_glosses.py, bundled beside EngineResources because host-api looks for them next to the resource directory. The directory is always created, empty when there are none, since the keyboard target bundles it as a folder; without the databases only English is glossed offline.
glosses_source=${2:-$repo_root/target/offline-glosses}
glosses_destination="$repo_root/target/ios/offline-glosses"
rm -rf "$glosses_destination"
mkdir -p "$glosses_destination"
if compgen -G "$glosses_source/zh-*.db" >/dev/null && [ -f "$glosses_source/offline-glosses-NOTICE.txt" ]; then
  cp "$glosses_source"/zh-*.db "$glosses_source/offline-glosses-NOTICE.txt" "$glosses_destination/"
  echo "offline glosses staged: $glosses_destination"
else
  echo "no offline glosses at $glosses_source; candidates are glossed offline in English only"
fi
echo "iOS resources staged from the pinned dictionary release: $destination"

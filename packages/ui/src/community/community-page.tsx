import { useState, type ReactNode } from "react";
import type { CustomSkinLibraryClient } from "../keyboard/touch-keyboard-skin-design";
import {
  CommunityHomePage,
  CommunityResourcesPage,
  type CommunityLocalDictionaryClient,
  type CommunityResourceClient,
  type CommunityResourceKind,
  type CommunityResourceScope,
} from "./community-resources";
import { CommunitySkinsPage, type CommunitySkinClient } from "./community-skins";
import {
  CommunityCandidateSkinsPage,
  type CandidateSkinCommunityClient,
} from "./community-candidate-skins";
import { CandidateSkinSyncStatus, useCandidateSkinSync } from "./candidate-skin-sync";
import { CommunityPluginsPage, type CommunityPluginClient } from "./community-plugins";
import type { PluginCatalogResult } from "../settings/plugins-section";
import * as style from "./community-style";
import { ExternalSkinDirectoryRow, useSkinCatalog, type SkinCatalog } from "../skin/external-skins";
import type { SkinImageReader } from "../skin/skin-image";
import { GroupList } from "../core/platform-controls";
import * as settings from "../settings/settings-style";

export interface CommunityPageProps {
  skins?: CommunitySkinClient;
  resources?: CommunityResourceClient;
  /** Desktop community commands for candidate-window skin packages. */
  candidateSkins?: CandidateSkinCommunityClient;
  /** Desktop community commands for plugin packs, shown as a second tab beside the candidate-window skins. */
  plugins?: CommunityPluginClient;
  /** The installed plugin packs, which the plugin gallery publishes from and checks before replacing one. */
  localPlugins?: () => Promise<PluginCatalogResult>;
  /** The installed external skins, which the candidate gallery publishes from and checks before replacing one. */
  localSkins?: () => Promise<SkinCatalog>;
  openSkinDirectory?: () => Promise<void>;
  /** Reads an installed package's images, so publishing can draw a preview for a package without one. */
  readSkinImage?: SkinImageReader;
  /** The host imports a skin the user points at instead of opening the skin directory (`HostSurface.skin_directory_import`). */
  importsSkin?: boolean;
  /** Opens 主题, where an installed candidate skin is enabled. */
  onOpenSkinPage?: () => void;
  theme: "light" | "dark";
  initialMine?: boolean;
  initialCategory?: "skin" | CommunityResourceKind;
  initialScope?: CommunityResourceScope;
  localDictionary?: CommunityLocalDictionaryClient;
  localSkinLibrary?: CustomSkinLibraryClient;
  mobile?: boolean;
  onLogin?: () => void;
  /** Resets the stateful gallery when the account destination changes. */
  destinationKey?: string;
}

/** The community page: the external skin directory, which community installs land in and which the signed-in user's library syncs with, above the gallery the host supports. A desktop host that also shares plugin packs gets a 候选窗皮肤 / 插件 tab strip, the skin directory belonging to the first tab only. */
export function CommunityPage(props: CommunityPageProps): ReactNode {
  const {
    skins,
    resources,
    candidateSkins,
    plugins,
    localPlugins,
    localSkins,
    openSkinDirectory,
    importsSkin = false,
    onLogin,
    destinationKey,
  } = props;
  const local = useSkinCatalog(localSkins, openSkinDirectory, importsSkin);
  const sync = useCandidateSkinSync(candidateSkins, () => void local.refresh());
  const [tab, setTab] = useState<"skin" | "plugin">("skin");
  if (!skins && !resources && !candidateSkins && !plugins) return null;
  const pluginGallery = plugins && (
    <CommunityPluginsPage
      key={destinationKey}
      client={plugins}
      localPlugins={localPlugins}
      onLogin={onLogin}
    />
  );
  if (!skins && !resources && !candidateSkins) return pluginGallery;
  const tabs = plugins && (
    <div className={style.categoryTabsPair} role="tablist" aria-label="社区分类">
      <button
        type="button"
        role="tab"
        aria-selected={tab === "skin"}
        onClick={() => setTab("skin")}
      >
        候选窗皮肤
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={tab === "plugin"}
        onClick={() => setTab("plugin")}
      >
        插件
      </button>
    </div>
  );
  if (tabs && tab === "plugin") {
    return (
      <div className={style.page}>
        {tabs}
        {pluginGallery}
      </div>
    );
  }
  const gallery = (
    <CommunityGallery
      {...props}
      onInstalled={() => {
        void local.refresh();
        sync.run();
      }}
    />
  );
  if (!localSkins && !openSkinDirectory) {
    return tabs ? (
      <div className={style.page}>
        {tabs}
        {gallery}
      </div>
    ) : (
      gallery
    );
  }
  return (
    <>
      {tabs && <div className={style.page}>{tabs}</div>}
      <div className={settings.groups}>
        <GroupList title="本地皮肤">
          <ExternalSkinDirectoryRow
            skins={local}
            scannable={!!localSkins}
            openable={!!openSkinDirectory}
            importsSkin={importsSkin}
            status={candidateSkins && <CandidateSkinSyncStatus sync={sync} />}
          />
        </GroupList>
      </div>
      {gallery}
    </>
  );
}

/** Selects the community surface supported by the host while preserving its destination state. */
function CommunityGallery({
  skins,
  resources,
  candidateSkins,
  localSkins,
  openSkinDirectory,
  readSkinImage,
  onOpenSkinPage,
  theme,
  initialMine = false,
  initialCategory = "skin",
  initialScope = "",
  localDictionary,
  localSkinLibrary,
  mobile = false,
  onLogin,
  destinationKey,
  onInstalled,
}: CommunityPageProps & { onInstalled: () => void }): ReactNode {
  if (skins && resources) {
    return (
      <CommunityHomePage
        key={destinationKey}
        skins={skins}
        resources={resources}
        theme={theme}
        initialMine={initialMine}
        initialCategory={initialCategory}
        initialScope={initialScope}
        localDictionary={localDictionary}
        localSkinLibrary={localSkinLibrary}
        mobile={mobile}
        onLogin={onLogin}
      />
    );
  }

  if (skins) {
    return (
      <CommunitySkinsPage
        key={destinationKey}
        client={skins}
        theme={theme}
        initialMine={initialMine}
        localSkinLibrary={localSkinLibrary}
        mobile={mobile}
        onLogin={onLogin}
      />
    );
  }

  if (resources) {
    return (
      <CommunityResourcesPage
        client={resources}
        kind={initialCategory === "reply" ? "reply" : "dictionary"}
        initialScope={initialScope}
        mobile={mobile}
      />
    );
  }

  if (candidateSkins) {
    return (
      <CommunityCandidateSkinsPage
        key={destinationKey}
        client={candidateSkins}
        localSkins={localSkins}
        openSkinDirectory={openSkinDirectory}
        readSkinImage={readSkinImage}
        onOpenSkinPage={onOpenSkinPage}
        onInstalled={onInstalled}
        onLogin={onLogin}
      />
    );
  }

  return null;
}

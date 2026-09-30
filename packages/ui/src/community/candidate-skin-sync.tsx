import { useEffect, useRef, useState } from "react";
import { candidateSkinMessage, communityNeedsSignIn } from "./community-helpers";
import type {
  CandidateSkinCommunityClient,
  CandidateSkinSyncReport,
} from "./community-candidate-skins";

export type CandidateSkinSyncState = {
  busy: boolean;
  report: CandidateSkinSyncReport | null;
  error: unknown;
  /** Starts a run, or queues one behind the run in progress so a change made meanwhile is not missed. */
  run: () => void;
};

/**
 * Keeps the local skin directory and the signed-in user's library in step: once as the page opens, and again whenever `run` is called. `onChanged` follows a run that downloaded or removed a local package, so a listing of the directory can scan again.
 */
export function useCandidateSkinSync(
  client: CandidateSkinCommunityClient | undefined,
  onChanged: () => void,
): CandidateSkinSyncState {
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<CandidateSkinSyncReport | null>(null);
  const [error, setError] = useState<unknown>(null);
  const generation = useRef(0);
  const running = useRef(false);
  const queued = useRef(false);
  const changed = useRef(onChanged);
  changed.current = onChanged;

  const start = async (current: number) => {
    if (!client) return;
    running.current = true;
    queued.current = false;
    setBusy(true);
    try {
      const result = await client.sync();
      if (current !== generation.current) return;
      setReport(result);
      setError(null);
      if (result.downloaded.length || result.deleted_local.length) changed.current();
    } catch (failure) {
      if (current !== generation.current) return;
      setError(failure);
    } finally {
      if (current === generation.current) {
        running.current = false;
        if (queued.current) void start(current);
        else setBusy(false);
      }
    }
  };

  useEffect(() => {
    const current = ++generation.current;
    running.current = false;
    queued.current = false;
    setReport(null);
    setError(null);
    setBusy(false);
    void start(current);
    return () => {
      generation.current++;
    };
  }, [client]);

  const run = () => {
    if (running.current) queued.current = true;
    else void start(generation.current);
  };

  return { busy, report, error, run };
}

/** Why one package was left out of a run, as a fixed sentence. */
function skipReason(code: string): string {
  if (code.startsWith("candidate_skin_") || code === "storage")
    return candidateSkinMessage({ code });
  switch (code) {
    case "account_invalid":
      return "服务器未接受这款皮肤。";
    case "account_conflict":
      return "云端的同名作品属于另一个皮肤包，未覆盖。";
    case "account_rate_limited":
      return "同步太频繁，下次再试。";
  }
  return "暂时无法同步，下次再试。";
}

function countLine(report: CandidateSkinSyncReport): string {
  const parts = [
    report.uploaded.length ? `上传 ${report.uploaded.length} 款` : "",
    report.downloaded.length ? `下载 ${report.downloaded.length} 款` : "",
    report.deleted_local.length ? `移除本地 ${report.deleted_local.length} 款` : "",
    report.deleted_cloud.length ? `移除云端 ${report.deleted_cloud.length} 款` : "",
  ].filter(Boolean);
  return parts.length ? `已同步：${parts.join("，")}。` : "本地皮肤已与云端皮肤库同步。";
}

function stoppedLine(code: string): string {
  return code === "candidate_skin_library_limit"
    ? "云端皮肤库已满 100 款，其余皮肤未上传。"
    : "同步太频繁，其余皮肤稍后再上传。";
}

/** The sync outcome under the 本地皮肤 row: what the last run did, why uploads stopped, and each package it left out. */
export function CandidateSkinSyncStatus({ sync }: { sync: CandidateSkinSyncState }) {
  if (sync.busy && !sync.report) {
    return (
      <span role="status" className="block">
        正在与云端皮肤库同步…
      </span>
    );
  }
  if (sync.error) {
    return (
      <span role="status" className="block">
        {communityNeedsSignIn(sync.error)
          ? "登录后，本地皮肤会自动同步到你的云端皮肤库，默认仅自己可见。"
          : `同步失败：${candidateSkinMessage(sync.error)}`}
      </span>
    );
  }
  const report = sync.report;
  if (!report) return null;
  return (
    <>
      <span role="status" className="block">
        {sync.busy ? "正在与云端皮肤库同步…" : countLine(report)}
        {report.stopped && ` ${stoppedLine(report.stopped)}`}
      </span>
      {report.skipped.length > 0 && (
        <details>
          <summary>{report.skipped.length} 款皮肤未同步</summary>
          <ul>
            {report.skipped.map((skip) => (
              <li key={skip.package_id}>
                {skip.package_id}：{skipReason(skip.code)}
              </li>
            ))}
          </ul>
        </details>
      )}
    </>
  );
}

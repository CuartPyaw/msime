import { AccountCloudBridge } from '../../account/AccountCloudBridge';
import { FileSessionStore, HarmonyAccountTransport } from '../../account/HarmonyAccountTransport';
import {
  CloudClipboardListing, CloudClipboardPolicy, CloudClipboardSendOutcome, CloudClipboardState
} from './CloudClipboardPolicy';

interface CloudClipboardListAction {
  operation: string;
  clipboard_operation: string;
  search: string;
}

interface CloudClipboardAddAction {
  operation: string;
  clipboard_operation: string;
  text: string;
}

/**
 * The keyboard extension's view of the account's cloud clipboard.
 *
 * It holds no credentials. Every operation opens the session file the settings application signed in with, in the `files/state` directory both processes share, and goes through the same `AccountCloudBridge` that page uses, so expiry, single-use refresh tokens and sign-out are handled in one place. Reading the file each time rather than keeping a bridge alive is what makes a sign-out or a different sign-in in the settings page take effect here without any notification between the processes.
 */
export class CloudClipboardClient {
  private readonly stateDirectory: string;
  private readonly transport: HarmonyAccountTransport = new HarmonyAccountTransport();

  constructor(stateDirectory: string) {
    this.stateDirectory = stateDirectory;
  }

  /** A signed-out keyboard answers from the session file alone and never touches the network for this. */
  async list(): Promise<CloudClipboardListing> {
    if (this.stateDirectory.length === 0) return { state: CloudClipboardState.FAILED, items: [] };
    const bridge: AccountCloudBridge = this.bridge();
    if (bridge.currentUserId() === null) return { state: CloudClipboardState.SIGNED_OUT, items: [] };
    const action: CloudClipboardListAction = { operation: 'clipboard', clipboard_operation: 'list', search: '' };
    return CloudClipboardPolicy.parseList(await bridge.handle(JSON.stringify(action)));
  }

  async add(text: string): Promise<CloudClipboardSendOutcome> {
    if (!CloudClipboardPolicy.validText(text)) return CloudClipboardSendOutcome.INVALID;
    if (this.stateDirectory.length === 0) return CloudClipboardSendOutcome.FAILED;
    const bridge: AccountCloudBridge = this.bridge();
    if (bridge.currentUserId() === null) return CloudClipboardSendOutcome.SIGNED_OUT;
    const action: CloudClipboardAddAction = { operation: 'clipboard', clipboard_operation: 'add', text: text };
    return CloudClipboardPolicy.parseSend(await bridge.handle(JSON.stringify(action)));
  }

  private bridge(): AccountCloudBridge {
    return new AccountCloudBridge(this.transport, new FileSessionStore(this.stateDirectory));
  }
}

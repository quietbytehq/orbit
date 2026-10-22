import { api, errorMessage, onTransfers } from "$lib/api";
import type { TransferItem, TransferRequest } from "$lib/api/types";

/** The transfer queue as the UI sees it. The backend pushes every change. */
class TransferStore {
  items = $state<TransferItem[]>([]);
  error = $state<string | null>(null);
  #started = false;

  /** Items still waiting or running. */
  get active(): TransferItem[] {
    return this.items.filter((item) => item.state.state === "queued" || item.state.state === "running");
  }

  get finished(): TransferItem[] {
    return this.items.filter((item) => !this.active.includes(item));
  }

  async start() {
    if (this.#started) return;
    this.#started = true;
    await onTransfers((items) => {
      this.items = items;
    });
    try {
      this.items = await api.listTransfers();
    } catch (err) {
      this.error = errorMessage(err);
    }
  }

  async enqueue(requests: TransferRequest[]): Promise<boolean> {
    this.error = null;
    try {
      await api.enqueueTransfers(requests);
      return true;
    } catch (err) {
      this.error = errorMessage(err);
      return false;
    }
  }

  async cancel(id: number) {
    try {
      await api.cancelTransfer(id);
    } catch (err) {
      this.error = errorMessage(err);
    }
  }

  async clearFinished() {
    await api.clearFinishedTransfers();
  }
}

export const transfers = new TransferStore();

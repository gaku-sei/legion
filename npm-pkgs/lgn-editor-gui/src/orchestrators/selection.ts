// This orchestrator manipulates several stores related to the current selection(s),
// that is, the element(s) selected in the viewport/resource browser/scene explorer.
//
// This orchestrator doesn't own any stores.
import { get } from "svelte/store";

import { MessageType } from "@lgn/proto-editor/dist/editor";

import { initMessageStream as initMessageStreamApi } from "@/api";
import { isEntry } from "@/lib/hierarchyTree";
import { fetchCurrentResourceDescription } from "@/orchestrators/currentResource";
import { fetchStagedResources } from "@/stores/stagedResources";

import {
  currentResourceDescriptionEntry,
  resourceEntries,
} from "./resourceBrowserEntries";

export function initMessageStream() {
  const subscription = initMessageStreamApi().subscribe(
    ({ lagging, message }) => {
      if (typeof lagging === "number") {
        // TODO: Handle lagging messages

        return;
      }

      if (message) {
        switch (message.msgType) {
          case MessageType.ResourceChanged: {
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            const resourceIds: string[] = JSON.parse(message.payload);

            const currentEntryValue = get(currentResourceDescriptionEntry);

            if (
              currentEntryValue &&
              resourceIds.indexOf(currentEntryValue.item.id) > -1
            ) {
              fetchCurrentResourceDescription(currentEntryValue.item.id, {
                notifySelection: false,
              }).catch(() => undefined); // TODO: Handle errors
            }

            fetchStagedResources().catch(() => undefined); // TODO: Handle errors
            break;
          }

          case MessageType.SelectionChanged: {
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            const resourceIds: string[] = JSON.parse(message.payload);

            // TODO: Catch error
            // TODO: Support multi-select (remove slice)
            if (!resourceIds.length) {
              currentResourceDescriptionEntry.set(null);

              return;
            }

            fetchCurrentResourceDescription(resourceIds[0], {
              notifySelection: false,
            })
              // TODO: Handle errors
              .catch(() => undefined);

            const selectedEntry = get(resourceEntries).find(
              (entry) => isEntry(entry) && resourceIds.includes(entry.item.id)
            );

            currentResourceDescriptionEntry.set(selectedEntry);
          }
        }
      }
    }
  );

  return () => subscription.unsubscribe();
}
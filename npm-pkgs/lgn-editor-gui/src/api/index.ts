import {
  Editor,
  PropertyInspector,
  ResourceBrowser,
  SourceControl,
} from "@lgn/apis/editor";
import { Log } from "@lgn/apis/log";
import { Runtime } from "@lgn/apis/runtime";
import { addAuthToClient } from "@lgn/web-client/src/lib/client";
import log from "@lgn/web-client/src/lib/log";

import { formatProperties } from "../components/propertyGrid/lib/propertyGrid";
import type {
  ResourcePropertyWithValue,
  ResourceWithProperties,
} from "../components/propertyGrid/lib/propertyGrid";

// const defaultGrpcEditorServerURL = "http://[::1]:50051";
// const defaultGrpcRuntimeServerURL = "http://[::1]:50052";
const defaultRestEditorServerURL = "http://[::1]:5051";
const defaultRestRuntimeServerURL = "http://[::1]:5052";

let resourceBrowserClient: ResourceBrowser.Client;

let propertyInspectorClient: PropertyInspector.Client;

let sourceControlClient: SourceControl.Client;

let editorClient: Editor.Client;

let runtimeClient: Runtime.Client;

let editorLogStreamClient: Log.Client;
let runtimeLogStreamClient: Log.Client;

export function initApiClient({
  // grpcEditorServerUrl = defaultGrpcEditorServerURL,
  // grpcRuntimeServerUrl = defaultGrpcRuntimeServerURL,
  restEditorServerUrl = defaultRestEditorServerURL,
  restRuntimeServerUrl = defaultRestRuntimeServerURL,
  accessTokenCookieName,
}: {
  grpcEditorServerUrl?: string;
  grpcRuntimeServerUrl?: string;
  restEditorServerUrl?: string;
  restRuntimeServerUrl?: string;
  accessTokenCookieName: string;
}) {
  resourceBrowserClient = new ResourceBrowser.Client({
    baseUri: restEditorServerUrl,
  });

  propertyInspectorClient = new PropertyInspector.Client({
    baseUri: restEditorServerUrl,
  });

  sourceControlClient = new SourceControl.Client({
    baseUri: restEditorServerUrl,
  });

  editorClient = new Editor.Client({
    baseUri: restEditorServerUrl,
  });

  runtimeClient = addAuthToClient(
    new Runtime.Client({ baseUri: restEditorServerUrl }),
    accessTokenCookieName
  );

  editorLogStreamClient = addAuthToClient(
    new Log.Client({ baseUri: restEditorServerUrl }),
    accessTokenCookieName
  );

  runtimeLogStreamClient = addAuthToClient(
    new Log.Client({ baseUri: restRuntimeServerUrl }),
    accessTokenCookieName
  );
}

/**
 * Eagerly fetches all the resource descriptions on the server
 * @returns All the resource descriptions
 */
export async function getAllResources(searchToken = "") {
  const resourceDescriptions: ResourceBrowser.ResourceDescription[] = [];

  async function getMoreResources(
    searchToken: string
  ): Promise<ResourceBrowser.ResourceDescription[]> {
    const response = await resourceBrowserClient.searchResources({
      params: { "space-id": "0", "workspace-id": "0", token: searchToken },
      query: {},
    });

    resourceDescriptions.push(...response.value.resource_descriptions);

    return response.value.next_search_token
      ? getMoreResources(response.value.next_search_token)
      : resourceDescriptions;
  }

  const allResources = await getMoreResources(searchToken);

  return allResources.sort((resource1, resource2) =>
    resource1.path > resource2.path ? 1 : -1
  );
}

export async function getRootResource(
  id: string
): Promise<ResourceBrowser.ResourceDescription | null> {
  const {
    value: {
      resource_descriptions: [resourceDescription],
    },
  } = await resourceBrowserClient.searchResources({
    params: { "space-id": "0", "workspace-id": "0", token: "" },
    query: { "root-resource-id": id },
  });

  return resourceDescription ?? null;
}

export async function getAllRootResources(ids: string[]) {
  const resources = await Promise.all(ids.map(getRootResource));

  return resources.filter(
    (resource): resource is ResourceBrowser.ResourceDescription => !!resource
  );
}

/**
 * Fetch a resource's properties using its ID
 * @param resource The resource description with the ID and the version
 * @returns The properties of the resource and possibly its description
 */
export async function getResourceProperties(
  id: string
): Promise<ResourceWithProperties> {
  const response = await propertyInspectorClient.getProperties({
    params: { "resource-id": id, "space-id": "0", "workspace-id": "0" },
  });

  if (response.type !== "200") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }

  const {
    value: { description, properties },
  } = response;

  return {
    id,
    description,
    version: description.version,
    properties: formatProperties(properties),
  };
}

export type PropertyUpdate = {
  name: string;
  // Can be any JSON serializable value
  value: ResourcePropertyWithValue["value"] | null;
};

/**
 * Update a resource's properties
 * @param resourceId The resource ID
 * @param version
 * @param propertyUpdates
 * @returns
 */
export async function updateResourceProperties(
  resourceId: string,
  version: number,
  propertyUpdates: PropertyUpdate[]
) {
  const response = await propertyInspectorClient.updateProperties({
    params: { "resource-id": resourceId, "space-id": "0", "workspace-id": "0" },
    body: {
      version,
      updates: propertyUpdates.map(({ name, value }) => ({
        name: name,
        // eslint-disable-next-line camelcase
        json_value: JSON.stringify(value),
      })),
    },
  });

  if (response.type !== "204") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }
}

/**
 * Update selection
 * @param resourceId The resource ID
 * @returns
 */
export async function updateSelection(resourceId: string) {
  const response = await propertyInspectorClient.updatePropertySelection({
    params: { "resource-id": resourceId, "space-id": "0", "workspace-id": "0" },
  });

  if (response.type !== "204") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }
}

export type AddVectorSubProperty = {
  path: string;
  index: number;
  jsonValue: string | undefined;
};

export async function addPropertyInPropertyVector(
  resourceId: string,
  { path, index, jsonValue }: AddVectorSubProperty
) {
  const response = await propertyInspectorClient.insertPropertyArrayItem({
    params: { "resource-id": resourceId, "space-id": "0", "workspace-id": "0" },
    // eslint-disable-next-line camelcase
    body: { array_path: path, index: BigInt(index), json_value: jsonValue },
  });

  if (response.type !== "200") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }

  const value = response.value.new_value;

  if (value) {
    window.dispatchEvent(
      new CustomEvent("refresh-property", {
        detail: { path, value },
      })
    );
  }
}

export type RemoveVectorSubProperty = {
  path: string;
  indices: number[];
};

export async function removeVectorSubProperty(
  resourceId: string,
  { path, indices }: RemoveVectorSubProperty
) {
  const response = await propertyInspectorClient.deletePropertiesArrayItem({
    params: { "resource-id": resourceId, "space-id": "0", "workspace-id": "0" },
    // eslint-disable-next-line camelcase
    body: { array_path: path, indices: indices.map(BigInt) },
  });

  if (response.type !== "204") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }
}

export async function getResourceTypes() {
  const response = await resourceBrowserClient.getResourceTypeNames({
    params: { "space-id": "0", "workspace-id": "0" },
  });

  return response.value;
}

export async function getAvailableComponentTypes() {
  const response = await propertyInspectorClient.getAvailableDynTraits({
    params: { "space-id": "0", "workspace-id": "0" },
    // eslint-disable-next-line camelcase
    query: { trait_name: "dyn Component" },
  });

  return response.value;
}

export async function createResource({
  resourceName,
  resourceType,
  parentResourceId,
  uploadId,
}: {
  resourceName: string;
  resourceType: string;
  parentResourceId: string | undefined;
  uploadId: string | undefined;
}) {
  const response = await resourceBrowserClient.createResource({
    params: { "space-id": "0", "workspace-id": "0" },
    body: {
      // eslint-disable-next-line camelcase
      resource_name: resourceName,
      // eslint-disable-next-line camelcase
      resource_type: resourceType,
      // eslint-disable-next-line camelcase
      // eslint-disable-next-line camelcase
      upload_id: uploadId,
      // eslint-disable-next-line camelcase
      parent_resource_id: parentResourceId,
      // eslint-disable-next-line camelcase
      init_values: [],
    },
  });

  return response.value;
}

export async function renameResource({
  id,
  newPath,
}: {
  id: string;
  newPath: string;
}) {
  await resourceBrowserClient.renameResource({
    params: { "space-id": "0", "workspace-id": "0" },
    // eslint-disable-next-line camelcase
    body: { new_path: newPath, id },
  });
}

export async function removeResource({ id }: { id: string }) {
  await resourceBrowserClient.deleteResource({
    params: { "space-id": "0", "workspace-id": "0" },
    body: { id },
  });
}

export async function cloneResource({
  sourceId,
  targetParentId,
}: {
  sourceId: string;
  targetParentId?: string;
}) {
  const response = await resourceBrowserClient.cloneResource({
    params: { "space-id": "0", "workspace-id": "0" },
    body: {
      // eslint-disable-next-line camelcase
      source_id: sourceId,
      // eslint-disable-next-line camelcase
      target_parent_id: targetParentId,
      // eslint-disable-next-line camelcase
      init_values: [],
    },
  });

  return response.value;
}

export async function revertResources({ ids }: { ids: string[] }) {
  await sourceControlClient.revertResources({
    params: { "space-id": "0", "workspace-id": "0" },
    body: ids,
  });
}

/**
 * Used for logging purpose
 * @param jsonCommand
 * @returns
 */
export function onSendEditionCommand(jsonCommand: string) {
  log.info("video", `Sending edition_command=${jsonCommand}`);
}

export async function initFileUpload({
  name,
  size,
}: {
  name: string;
  size: number;
}) {
  const response = await sourceControlClient.contentUploadInit({
    params: { "space-id": "0", "workspace-id": "0" },
    body: { name, size: BigInt(size) },
  });

  if (response.type !== "200") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }

  return response.value;
}

export async function streamFileUpload({
  id,
  content,
}: {
  id: string;
  content: Blob;
}): Promise<SourceControl.ContentUploadSucceeded> {
  const response = await sourceControlClient.contentUpload({
    params: { "space-id": "0", "workspace-id": "0", "transaction-id": id },
    body: content,
  });

  if (response.type !== "200") {
    throw new Error(`Request was not successful: ${JSON.stringify(response)}`);
  }

  return response.value;
}

// FIXME: This function is known for being broken
// the api is not fully over yet and it might change soon
export async function reparentResources({
  id,
  newPath,
}: {
  id: string;
  newPath: string;
}) {
  await resourceBrowserClient.reparentResource({
    params: { "space-id": "0", "workspace-id": "0" },
    // eslint-disable-next-line camelcase
    body: { id, new_path: newPath },
  });
}

export async function syncLatest() {
  await sourceControlClient.syncLatest({
    params: { "space-id": "0", "workspace-id": "0" },
  });
}

export async function commitStagedResources({ message }: { message: string }) {
  await sourceControlClient.commitStagedResources({
    params: { "space-id": "0", "workspace-id": "0" },
    body: { message },
  });
}

export async function getStagedResources() {
  const response = await sourceControlClient.getStagedResources({
    params: { "space-id": "0", "workspace-id": "0" },
  });

  return response.value;
}

export async function openScene({ id }: { id: string }) {
  await resourceBrowserClient.openScene({
    // eslint-disable-next-line camelcase
    params: { "space-id": "0", "workspace-id": "0", scene_id: id },
  });
}

export async function closeScene({ id }: { id: string }) {
  await resourceBrowserClient.closeScene({
    // eslint-disable-next-line camelcase
    params: { "space-id": "0", "workspace-id": "0", scene_id: id },
  });
}

export async function getActiveSceneIds() {
  const response = await resourceBrowserClient.getActiveScenes({
    params: { "space-id": "0", "workspace-id": "0" },
  });

  return response.value.scene_ids;
}

export async function getRuntimeSceneInfo({
  resourceId,
}: {
  resourceId: string;
}) {
  const response = await resourceBrowserClient.getRuntimeSceneInfo({
    params: { "space-id": "0", "workspace-id": "0", "resource-id": resourceId },
  });

  return response.value;
}

export async function getActiveScenes() {
  return getAllRootResources(await getActiveSceneIds());
}

export function getEditorTraceEvents() {
  return editorLogStreamClient.logEntries({
    params: { "space-id": "0", "workspace-id": "0" },
  });
}

export function getRuntimeTraceEvents() {
  return runtimeLogStreamClient.logEntries({
    params: { "space-id": "0", "workspace-id": "0" },
  });
}

export async function getLastMessage() {
  const response = await editorClient.getMessages({
    params: { "space-id": "0", "workspace-id": "0" },
  });

  return response.type === "200" ? response.value : null;
}

export async function loadRuntimeManifest({
  manifestId,
}: {
  manifestId: string;
}) {
  return runtimeClient.loadManifest({
    params: { "space-id": "0", "workspace-id": "0" },
    body: new Blob([manifestId]),
  });
}

export async function loadRuntimeRootAsset({
  rootAssetId,
}: {
  rootAssetId: string;
}) {
  return runtimeClient.loadRootAsset({
    params: { "space-id": "0", "workspace-id": "0" },
    body: new Blob([rootAssetId]),
  });
}

export async function pauseRuntime() {
  return runtimeClient.pause({
    params: { "space-id": "0", "workspace-id": "0" },
  });
}

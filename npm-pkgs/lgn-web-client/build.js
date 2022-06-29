import apiCodegen from "@lgn/vite-plugin-api-codegen";

apiCodegen({
  aliasMappings: {
    "../../crates/lgn-governance/apis/space.yaml": "Space",
    "../../crates/lgn-governance/apis/workspace.yaml": "Workspace",
  },
  apiOptions: [
    {
      path: "../../crates/lgn-streamer/apis",
      names: ["streaming"],
      filename: "streaming",
    },
    {
      path: "../../crates/lgn-log/apis",
      names: ["log"],
      filename: "log",
    },
    {
      path: "../../crates/lgn-editor-srv/apis",
      names: [
        "editor",
        "property_inspector",
        "resource_browser",
        "source_control",
      ],
      filename: "editor",
    },
  ],
})
  .buildStart()
  .catch(console.error);
